use oauth2::{AuthorizationCode, CsrfToken};
use serde::{Deserialize, Serialize};
use tiny_http::{Response, Server, SslConfig};
use url::Url;

use super::Authorizer;

use crate::KResult;

/// BrowserAuthorizer is an authorizer that opens your browser to
/// login to the authorizer, then starts an http server locally to
/// capture the AuthorizationCode. The redirecturl **must be**
/// http://127.0.0.1:8085 for this to work.
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct BrowserAuthorizer {
    pub redirect_port: u16,
    pub redirect_certificate: Option<Vec<u8>>,
    pub redirect_private_key: Option<Vec<u8>>,
}

impl BrowserAuthorizer {
    fn is_ssl(&self) -> bool {
        return self.redirect_certificate.is_some() && self.redirect_private_key.is_some();
    }
}

impl Authorizer for BrowserAuthorizer {
    fn authorize(&self, url: Url, _csrf: CsrfToken) -> KResult<AuthorizationCode> {
        webbrowser::open(url.as_str())?;

        let server = if self.is_ssl() {
            Server::https(
                format!("127.0.0.1:{}", self.redirect_port),
                SslConfig {
                    certificate: self
                        .redirect_certificate
                        .clone()
                        .expect("is_ssl already checks this"),
                    private_key: self
                        .redirect_private_key
                        .clone()
                        .expect("is ssl already checks this"),
                },
            )?
        } else {
            Server::http(format!("127.0.0.1:{}", self.redirect_port))?
        };

        // Block until we receive one request
        let request = server.recv()?;

        // Reconstruct the full URL so we can parse query params
        let full_url = format!("http://127.0.0.1{}", request.url());
        let parsed = Url::parse(&full_url)?;

        // Extract the `code` query param from ?code=...&state=...
        let code = parsed
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, value)| value.into_owned())
            .ok_or("No 'code' param in callback URL")?;

        // Respond to the browser so the user sees a success page
        let response = Response::from_string(
        "<html><body><h2>Authentication successful!</h2><p>You can close this tab.</p></body></html>"
    ).with_header(
        "Content-Type: text/html".parse::<tiny_http::Header>().unwrap()
    );
        request.respond(response)?;

        Ok(AuthorizationCode::new(code))
    }
}
