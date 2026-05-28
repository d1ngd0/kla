use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::{BasicAuth, OAuth, SigV4};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum Authentication {
    #[serde(rename = "sigv4")]
    SigV4(SigV4),
    #[serde(rename = "oauth")]
    OAuth(OAuth),
    #[serde(rename = "basic")]
    BasicAuth(BasicAuth),
    #[serde(rename = "none")]
    None,
}

impl Default for Authentication {
    fn default() -> Self {
        return Authentication::None;
    }
}

impl Authentication {
    pub fn resolve_working_dir<P: AsRef<Path>>(&mut self, dir: P) {
        match self {
            Authentication::SigV4(_) => (),
            Authentication::OAuth(oauth) => oauth.resolve_working_dir(dir.as_ref()),
            Authentication::BasicAuth(basic) => basic.resolve_working_dir(dir.as_ref()),
            Authentication::None => (),
        }
    }
}
