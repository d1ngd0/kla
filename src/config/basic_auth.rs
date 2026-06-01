use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::SecretValue;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BasicAuth {
    pub username: SecretValue,
    pub password: Option<SecretValue>,
}

impl BasicAuth {
    pub fn resolve_working_dir<P: AsRef<Path>>(&mut self, dir: P) {
        self.password
            .as_mut()
            .map(|f| f.resolve_working_dir(dir.as_ref()));
    }
}
