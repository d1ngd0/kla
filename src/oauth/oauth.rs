use std::{
    fs::{self, read_to_string},
    ops::Add as _,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use chrono::{Duration, Local};
use oauth2::{
    basic::BasicClient, AccessToken, AuthUrl, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    RedirectUrl, Scope, TokenResponse as _, TokenUrl,
};
use oauth2_reqwest::ReqwestBlockingClient;
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::Result;

use super::{token_file_contents::TokenFileContents, Authorizer};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAuth<T: Authorizer> {
    client_id: ClientId,
    client_secret: ClientSecret,
    authorization_url: AuthUrl,
    token_url: TokenUrl,
    #[serde(default)]
    scopes: Vec<Scope>,
    #[serde(skip, default)]
    token_contents: Arc<RwLock<TokenFileContents>>,
    #[serde(skip, default)]
    authorizer: T,
}

/// implment the TryFrom trait for any Authorizer that specifies a default
/// value.
impl<T: Authorizer + Default> TryFrom<crate::config::OAuth> for OAuth<T> {
    type Error = crate::Error;

    fn try_from(value: crate::config::OAuth) -> std::result::Result<Self, Self::Error> {
        let s = Self {
            client_id: value.client_id,
            client_secret: value.client_secret.try_into()?,
            authorization_url: value.authorization_url,
            token_url: value.token_url,
            scopes: value.scopes,
            token_contents: Arc::new(RwLock::new(TokenFileContents::default())),
            authorizer: T::default(),
        };
        Ok(s)
    }
}

impl<T: Authorizer> OAuth<T> {
    /// sign will take the existing token and apply it
    /// to the header.
    pub fn authorize(&self, builder: RequestBuilder) -> Result<RequestBuilder> {
        let token = self.fetch_token()?;
        Ok(builder.bearer_auth(token.into_secret()))
    }

    pub fn fetch_token(&self) -> Result<AccessToken> {
        // First check if the contents are in memory, if they are and aren't expired let's return that
        let token_contents = self.token_contents.read().unwrap();
        if let Some(token) = token_contents.token() {
            return Ok(token.clone());
        }
        drop(token_contents);

        // The reading stuff is done, we want to make sure we are the only ones doing things
        // now, so lets get to work by fetching a write lock
        let mut token_contents = self.token_contents.write().unwrap();

        // it's possible more than one of us got to this point, so let's make sure
        // a sibling process didn't already update the contents, if they did we
        // can return and unlock
        if let Some(token) = token_contents.token() {
            return Ok(token.clone());
        }

        // Alright, the contents are not in memory, or the data is old, let's try
        // to fetch it from the disk, lets generate the name
        let path = self.token_filename();

        // Now check if the file exists, if it does we can load it into memory, and return the value
        if path.exists() {
            let contents = read_to_string(&path)?;
            let file_contents: TokenFileContents = serde_json::from_str(contents.as_str())?;

            if !file_contents.expired() {
                token_contents.update(file_contents);
                return Ok(token_contents
                    .token()
                    .expect("We litterally just set the token and it wasn't expired")
                    .clone());
            }
        }

        // Ok the path doesn't exist, so we need to go through the whole oauth process
        let process_contents = self.token()?;
        fs::write(&path, serde_json::to_string(&process_contents)?)?;

        token_contents.update(process_contents);

        return Ok(token_contents
            .token()
            .expect("We litterally just set the token and it wasn't expired")
            .clone());
    }

    pub fn token_filename(&self) -> PathBuf {
        let mut hasher = Sha256::new();

        hasher.update(self.client_id.as_bytes());
        hasher.update(self.client_secret.secret());
        hasher.update(self.authorization_url.as_bytes());
        hasher.update(self.token_url.as_bytes());

        for scope in &self.scopes {
            hasher.update(scope.as_bytes());
        }

        // To see it as a hex string
        PathBuf::from("/tmp").join(format!("{}.token", hex::encode(hasher.finalize())))
    }

    pub fn token(&self) -> Result<TokenFileContents> {
        // Create an OAuth2 client by specifying the client ID, client secret, authorization URL and
        // token URL.
        let client = BasicClient::new(self.client_id.clone())
            .set_client_secret(self.client_secret.clone())
            .set_auth_uri(self.authorization_url.clone())
            .set_token_uri(self.token_url.clone())
            // Set the URL the user will be redirected to after the authorization process.
            .set_redirect_uri(RedirectUrl::new("http://127.0.0.1:8085".into())?);

        // Generate a PKCE challenge.
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        // Generate the full authorization URL.
        let (auth_url, csrf_token) = client
            .authorize_url(CsrfToken::new_random)
            .add_scopes(self.scopes.clone())
            // Set the PKCE code challenge.
            .set_pkce_challenge(pkce_challenge)
            .url();

        // This is the URL you should redirect the user to, in order to trigger the authorization
        // process.
        let auth_code = self.authorizer.authorize(auth_url, csrf_token)?;

        // Once the user has been redirected to the redirect URL, you'll have access to the
        // authorization code. For security reasons, your code should verify that the `state`
        // parameter returned by the server matches `csrf_token`.

        let http_client = reqwest::blocking::ClientBuilder::new()
            // Following redirects opens the client up to SSRF vulnerabilities.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Client should build");
        let http_client = ReqwestBlockingClient::from(http_client);

        // Now you can trade it for an access token.
        let token_resp = client
            .exchange_code(auth_code)
            // Set the PKCE code verifier.
            .set_pkce_verifier(pkce_verifier)
            .request(&http_client)?;

        Ok(TokenFileContents::new(
            token_resp.access_token().clone(),
            Local::now().add(
                token_resp
                    .expires_in()
                    .map(Duration::from_std)
                    .map(|f| f.ok())
                    .flatten()
                    .unwrap_or_else(|| Duration::seconds(300)),
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, RwLock};

    use http::Method;
    use httpmock::MockServer;
    use oauth2::{AuthUrl, AuthorizationCode, ClientId, ClientSecret, Scope, TokenUrl};
    use reqwest::ClientBuilder;
    use serde_json::json;
    use tokio::runtime::Runtime;

    use super::{OAuth, TokenFileContents};

    type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn test() -> TestResult<()> {
        let tmpdir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&tmpdir).unwrap();
        // Start mock server
        let server = MockServer::start();

        //
        // Mock authorization endpoint
        //
        //let authorize_mock = server.mock(|when, then| {
        //    when.method("POST").path("/oauth/v2/authorize");

        //    then.status(302).header(
        //        "location",
        //        "http://localhost:3000/callback?code=test-auth-code&state=xyz",
        //    );
        //});

        //
        // Mock OAuth token endpoint
        //
        let token_mock = server.mock(|when, then| {
            when.method("POST").path("/oauth/v2/token");

            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "access_token": "mock-access-token",
                    "token_type": "Bearer",
                    "expires_in": 3600,
                    "scope": "read write"
                }));
        });

        //
        // Mock protected API endpoint
        //
        let api_mock = server.mock(|when, then| {
            when.method("GET").path("/api/userinfo");

            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "id": 123,
                    "username": "test-user",
                    "email": "test@example.com"
                }));
        });

        //
        // Example URLs your OAuth client would use
        //
        let auth_url = server.url("/oauth/v2/authorize");
        let token_url = server.url("/oauth/v2/token");
        let api_url = server.url("/api/userinfo");

        let auth = OAuth {
            client_id: ClientId::new("client_id".into()),
            client_secret: ClientSecret::new("2io3fnaldvmaw09evmaisdhfas".into()),
            authorization_url: AuthUrl::new(auth_url.into())?,
            token_url: TokenUrl::new(token_url.into())?,
            token_contents: Arc::new(RwLock::new(TokenFileContents::default())),
            scopes: vec![
                Scope::new("read".to_string()),
                Scope::new("write".to_string()),
            ],
            authorizer: |_, _| Ok(AuthorizationCode::new("test-auth-code".into())),
        };

        let client = ClientBuilder::new().build()?;
        let mut builder = client.request(Method::GET, api_url);
        builder = auth.authorize(builder)?;
        let req = builder.build()?;

        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            match client.execute(req).await {
                Ok(resp) => assert!(
                    resp.status().is_success(),
                    "request failed {:?}",
                    resp.status()
                ),
                Err(err) => assert!(false, "{}", err),
            }
        });

        token_mock.assert();
        // authorize_mock.assert();
        api_mock.assert();

        Ok(())
    }
}
