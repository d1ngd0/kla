use chrono::{DateTime, Local};
use oauth2::AccessToken;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TokenFileContents {
    token: AccessToken,
    expires: DateTime<Local>,
}

impl Default for TokenFileContents {
    fn default() -> Self {
        Self {
            token: AccessToken::new(String::default()),
            expires: Local::now(),
        }
    }
}

/// TokenFileContents stores the contents of the token file
impl TokenFileContents {
    /// new creates a new token file contents
    pub fn new(token: AccessToken, expires: DateTime<Local>) -> TokenFileContents {
        Self { token, expires }
    }

    /// empty returns if the token has been set or not
    pub fn empty(&self) -> bool {
        return self.token.secret() == "";
    }

    /// expired returns true when the token is expired
    pub fn expired(&self) -> bool {
        self.expires <= Local::now()
    }

    /// update consumes a TokenFileContents and sets its
    /// values to the values specified in from.
    pub fn update(&mut self, from: TokenFileContents) {
        self.token = from.token;
        self.expires = from.expires
    }

    /// token returns a reference to the AccessToken if it
    /// is set and not expired, otherwise it returns None.
    pub fn token(&self) -> Option<&AccessToken> {
        if self.empty() {
            return None;
        }

        if self.expired() {
            return None;
        }

        Some(&self.token)
    }
}
