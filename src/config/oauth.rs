use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
};

use oauth2::{AuthUrl, ClientId, ClientSecret as OauthClientSecret, Scope, TokenUrl};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OAuth {
    pub client_id: ClientId,
    pub client_secret: ClientSecret,
    pub authorization_url: AuthUrl,
    pub token_url: TokenUrl,
    #[serde(default)]
    pub scopes: Vec<Scope>,
}

impl OAuth {
    /// resolve_working_dir finds any relative paths referenced in the config
    /// and resolves them with `dir` as it's base.
    pub fn resolve_working_dir<P: AsRef<Path>>(&mut self, dir: P) {
        match &self.client_secret {
            ClientSecret::File(path) => {
                if path.is_relative() {
                    self.client_secret = ClientSecret::File(PathBuf::from(dir.as_ref()).join(path))
                }
            }
            ClientSecret::Value(_) => (),
        }
    }
}

/// ClientSecret holds a file path or a value to specify the client secret.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum ClientSecret {
    #[serde(rename = "file")]
    File(PathBuf),
    #[serde(rename = "value")]
    Value(OauthClientSecret),
}

impl TryFrom<ClientSecret> for oauth2::ClientSecret {
    type Error = crate::Error;

    /// Consume the ClientSecret and return an oauth::ClientSecret
    fn try_from(value: ClientSecret) -> Result<Self, Self::Error> {
        match value {
            ClientSecret::File(path) => Ok(oauth2::ClientSecret::new(read_to_string(path)?)),
            ClientSecret::Value(client_secret) => Ok(client_secret),
        }
    }
}

#[cfg(test)]
mod test {
    use oauth2::{AuthUrl, ClientId, Scope, TokenUrl};

    use crate::OAuth;

    type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn test_serialization() -> TestResult<()> {
        let s = r#"
        {
            "client_id": "testvalue",
            "client_secret": {
                "file": "/tmp/something"
            },
            "authorization_url": "https://localhost:9999",
            "token_url": "https://localhost:9999",
            "scopes": ["testvalue"]
        }
        "#;

        let oauth_config: OAuth = serde_json::from_str(&s)?;
        let expected = OAuth {
            client_id: ClientId::new("testvalue".into()),
            client_secret: crate::ClientSecret::File("/tmp/something".into()),
            authorization_url: AuthUrl::new("https://localhost:9999".into())?,
            token_url: TokenUrl::new("https://localhost:9999".into())?,
            scopes: vec![Scope::new("testvalue".into())],
        };

        assert_eq!(oauth_config.client_id, expected.client_id);
        assert_eq!(oauth_config.authorization_url, expected.authorization_url);
        assert_eq!(oauth_config.token_url, expected.token_url);
        assert_eq!(oauth_config.scopes, expected.scopes);

        match (oauth_config.client_secret, expected.client_secret) {
            (crate::ClientSecret::File(path_a), crate::ClientSecret::File(path_b)) => {
                assert_eq!(path_a, path_b)
            }
            (crate::ClientSecret::File(_), crate::ClientSecret::Value(_)) => {
                assert!(false, "got File and Value")
            }
            (crate::ClientSecret::Value(_), crate::ClientSecret::File(_)) => {
                assert!(false, "got Value and File")
            }
            (crate::ClientSecret::Value(csa), crate::ClientSecret::Value(csb)) => {
                assert_eq!(csa.secret(), csb.secret())
            }
        }

        Ok(())
    }
}
