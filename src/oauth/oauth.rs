use std::path::PathBuf;

use chrono::Duration;
use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
    Scope, TokenResponse as _, TokenUrl,
};
use oauth2_reqwest::ReqwestBlockingClient;
use rcgen::{generate_simple_self_signed, CertifiedKey};
use reqwest::RequestBuilder;
use sha2::{Digest as _, Sha256};

use crate::{filecache::CacheFile, oauth::BrowserAuthorizer, Authentication, Result};

use super::Authorizer;

#[derive(Clone)]
pub struct OAuth<T: Authorizer> {
    client_id: ClientId,
    client_secret: ClientSecret,
    authorization_url: AuthUrl,
    token_url: TokenUrl,
    redirect_url: RedirectUrl,
    scopes: Vec<Scope>,
    token: CacheFile,
    authorizer: T,
}

impl<T: Authorizer> std::fmt::Debug for OAuth<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuth")
            .field("client_id", &self.client_id)
            .field("client_secret", &self.client_secret)
            .field("authorization_url", &self.authorization_url)
            .field("token_url", &self.token_url)
            .field("redirect_url", &self.redirect_url)
            .field("scopes", &self.scopes)
            .field("token", &self.token)
            .finish()
    }
}

/// implment the TryFrom trait for any Authorizer that specifies a default
/// value.
impl TryFrom<crate::config::OAuth> for OAuth<BrowserAuthorizer> {
    type Error = crate::Error;

    fn try_from(value: crate::config::OAuth) -> std::result::Result<Self, Self::Error> {
        let redirect_port = value.redirect_port.unwrap_or(8085);
        let mut auth = BrowserAuthorizer {
            redirect_port: redirect_port,
            redirect_certificate: None,
            redirect_private_key: None,
        };

        if value.https {
            let CertifiedKey { cert, signing_key } =
                generate_simple_self_signed(["127.0.0.1".into(), "localhost".into()])?;
            auth.redirect_certificate = Some(cert.pem().into());
            auth.redirect_private_key = Some(signing_key.public_key_pem().into());
        }

        let token_name = Self::token_filename(
            &value.client_id,
            &value.authorization_url,
            &value.token_url,
            &value.scopes,
        );

        let s = Self {
            client_id: value.client_id,
            client_secret: value.client_secret.try_into()?,
            authorization_url: value.authorization_url,
            token_url: value.token_url,
            redirect_url: RedirectUrl::new(format!(
                "{}://127.0.0.1:{}",
                if value.https { "https" } else { "http" },
                redirect_port
            ))?,
            scopes: value.scopes,
            token: CacheFile::new(token_name),
            authorizer: auth,
        };
        Ok(s)
    }
}

impl<T: Authorizer + Sync + Send> Authentication for OAuth<T> {
    fn authorize(&self, builder: RequestBuilder) -> Result<RequestBuilder> {
        let token = self.token.fetch(|| self.oauth_flow())?;
        Ok(builder.bearer_auth(token))
    }
}

impl<T: Authorizer> OAuth<T> {
    pub fn token_filename(
        client: &ClientId,
        auth: &AuthUrl,
        token: &TokenUrl,
        scopes: &[Scope],
    ) -> PathBuf {
        let mut hasher = Sha256::new();

        hasher.update(client.as_bytes());
        hasher.update(auth.as_bytes());
        hasher.update(token.as_bytes());

        for scope in scopes {
            hasher.update(scope.as_bytes());
        }

        // To see it as a hex string
        PathBuf::from("/tmp").join(format!("{}.token", hex::encode(hasher.finalize())))
    }

    pub fn oauth_flow(&self) -> Result<(String, chrono::Duration)> {
        // Create an OAuth2 client by specifying the client ID, client secret, authorization URL and
        // token URL.
        let client = BasicClient::new(self.client_id.clone())
            .set_client_secret(self.client_secret.clone())
            .set_auth_uri(self.authorization_url.clone())
            .set_token_uri(self.token_url.clone())
            .set_redirect_uri(self.redirect_url.clone());

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

        Ok((
            token_resp.access_token().clone().into_secret(),
            token_resp
                .expires_in()
                .map(Duration::from_std)
                .map(|f| f.ok())
                .flatten()
                .unwrap_or_else(|| Duration::seconds(300)),
        ))
    }
}

#[cfg(test)]
mod tests {

    use http::Method;
    use httpmock::MockServer;
    use oauth2::{
        AuthUrl, AuthorizationCode, ClientId, ClientSecret, RedirectUrl, Scope, TokenUrl,
    };
    use reqwest::ClientBuilder;
    use serde_json::json;
    use tokio::runtime::Runtime;

    use crate::{filecache::CacheFile, Authentication as _};

    use super::OAuth;

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
            token: CacheFile::new("./test"),
            redirect_url: RedirectUrl::new("http://localhost:8085".into())?,
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
