use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::ValueSource;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ROPC {
    pub client_id: ValueSource,
    pub token_url: String,
    pub client_secret: ValueSource,
    pub username: ValueSource,
    pub password: ValueSource,
}

impl ROPC {
    /// resolve_working_dir finds any relative paths referenced in the config
    /// and resolves them with `dir` as it's base.
    pub fn resolve_working_dir<P: AsRef<Path>>(&mut self, dir: P) {
        self.client_secret.resolve_working_dir(dir.as_ref());
        self.username.resolve_working_dir(dir.as_ref());
        self.password.resolve_working_dir(dir.as_ref());
    }
}
