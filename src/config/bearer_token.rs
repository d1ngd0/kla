use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::FileOrValue;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BearerToken {
    pub token: FileOrValue,
}

impl BearerToken {
    pub fn resolve_working_dir<P: AsRef<Path>>(&mut self, dir: P) {
        self.token.resolve_working_dir(dir.as_ref());
    }
}
