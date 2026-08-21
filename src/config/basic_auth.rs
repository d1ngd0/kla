use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::ValueSource;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BasicAuth {
    pub username: ValueSource,
    pub password: Option<ValueSource>,
}

impl BasicAuth {
    pub fn resolve_working_dir<P: AsRef<Path>>(&mut self, dir: P) {
        self.password
            .as_mut()
            .map(|f| f.resolve_working_dir(dir.as_ref()));
    }
}
