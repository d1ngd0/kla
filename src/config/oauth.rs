use std::path::Path;

use oauth2::{AuthUrl, ClientId, Scope, TokenUrl};
use serde::{Deserialize, Serialize};

use crate::config::SecretValue;

// TODO: this should be more configurable, we assume they will use the browser
// authorizer, but there could be better ways to do this in the future, like a
// proxy that we should support.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OAuth {
    pub client_id: ClientId,
    pub client_secret: ClientSecret,
    pub authorization_url: AuthUrl,
    pub redirect_port: Option<u16>,
    #[serde(default)]
    pub https: bool,
    pub token_url: TokenUrl,
    #[serde(default)]
    pub scopes: Vec<Scope>,
}

impl OAuth {
    /// resolve_working_dir finds any relative paths referenced in the config
    /// and resolves them with `dir` as it's base.
    pub fn resolve_working_dir<P: AsRef<Path>>(&mut self, dir: P) {
        self.client_secret.resolve_working_dir(dir.as_ref())
    }
}

/// ClientSecret holds a file path or a value to specify the client secret.
pub type ClientSecret = SecretValue;

impl TryFrom<ClientSecret> for oauth2::ClientSecret {
    type Error = crate::Error;

    /// Consume the ClientSecret and return an oauth::ClientSecret
    fn try_from(value: ClientSecret) -> Result<Self, Self::Error> {
        Ok(oauth2::ClientSecret::new(value.try_into()?))
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use oauth2::{AuthUrl, ClientId, Scope, TokenUrl};

    use super::OAuth;

    type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn test_serialization() -> TestResult<()> {
        let s = r#"
        {
            "client_id": "testvalue",
            "client_secret": "something",
            "authorization_url": "https://localhost:9999",
            "token_url": "https://localhost:9999",
            "scopes": ["testvalue"]
        }
        "#;

        let oauth_config: OAuth = serde_json::from_str(&s)?;
        let expected = OAuth {
            client_id: ClientId::new("testvalue".into()),
            client_secret: super::ClientSecret::Value("something".into()),
            authorization_url: AuthUrl::new("https://localhost:9999".into())?,
            token_url: TokenUrl::new("https://localhost:9999".into())?,
            scopes: vec![Scope::new("testvalue".into())],
            redirect_port: None,
            https: false,
        };

        assert_eq!(oauth_config.client_id, expected.client_id);
        assert_eq!(oauth_config.authorization_url, expected.authorization_url);
        assert_eq!(oauth_config.token_url, expected.token_url);
        assert_eq!(oauth_config.scopes, expected.scopes);
        assert_eq!(
            String::try_from(oauth_config.client_secret)?,
            String::try_from(expected.client_secret)?
        );

        Ok(())
    }
}
