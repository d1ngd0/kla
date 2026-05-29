use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::SecretValue;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BasicAuth {
    pub username: Option<String>,
    pub password: Option<SecretValue>,
    pub userpass: Option<SecretValue>,
}

impl BasicAuth {
    pub fn resolve_working_dir<P: AsRef<Path>>(&mut self, dir: P) {
        self.password
            .as_mut()
            .map(|f| f.resolve_working_dir(dir.as_ref()));
        self.userpass
            .as_mut()
            .map(|f| f.resolve_working_dir(dir.as_ref()));
    }
}
