use std::{
    fmt::Display,
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

    /// into_inner returns the inner object
    pub fn into_inner(self) -> Vec<Extension> {
        self.extensions
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

    /// lock will stop the extension from updating any further
    #[serde(default)]
    pub lock: bool,
}

impl Display for Extension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.lock {
            write!(
                f,
                "{} LOCKED @<{}>",
                self.remote,
                self.dir.to_string_lossy()
            )
        } else {
            write!(f, "{} @<{}>", self.remote, self.dir.to_string_lossy())
        }
    }
}

impl AsRef<Reference> for Extension {
    fn as_ref(&self) -> &Reference {
        &self.remote
    }
}

impl Eq for Extension {}
impl PartialEq for Extension {
    fn eq(&self, other: &Self) -> bool {
        self.remote.registry() == other.remote.registry()
            && self.remote.repository() == other.remote.repository()
    }
}
impl PartialEq<Reference> for Extension {
    fn eq(&self, other: &Reference) -> bool {
        self.remote.registry() == other.registry() && self.remote.repository() == other.repository()
    }
}

impl TryFrom<&Extension> for Config {
    type Error = crate::Error;

    fn try_from(value: &Extension) -> Result<Self, Self::Error> {
        Config::from_path(value.dir.join(EXTENSION_ROOT))
    }
}
