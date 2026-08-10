use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use oci_client::Reference;
use serde::{Deserialize, Serialize};

use crate::config::Config;

pub(crate) const EXTENSION_ROOT: &'static str = "config.toml";

#[derive(Deserialize, Serialize)]
pub struct ExtensionSet {
    #[serde(rename = "extension")]
    extensions: Vec<Extension>,
}

impl ExtensionSet {
    pub fn empty() -> Self {
        ExtensionSet { extensions: vec![] }
    }
}

impl Deref for ExtensionSet {
    type Target = Vec<Extension>;

    fn deref(&self) -> &Self::Target {
        &self.extensions
    }
}

impl DerefMut for ExtensionSet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.extensions
    }
}

#[derive(Deserialize, Serialize)]
pub struct Extension {
    // relative path to the extension we want to load
    pub dir: PathBuf,

    // where we originally pulled this value from
    pub remote: Reference,
}

impl Eq for Extension {}
impl PartialEq for Extension {
    fn eq(&self, other: &Self) -> bool {
        self.remote.registry() == other.remote.registry()
            && self.remote.repository() == other.remote.repository()
    }
}

impl TryFrom<&Extension> for Config {
    type Error = crate::Error;

    fn try_from(value: &Extension) -> Result<Self, Self::Error> {
        Config::from_path(value.dir.join(EXTENSION_ROOT))
    }
}
