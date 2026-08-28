use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::Authentication;

/// Extensions holds the configuration for extensions
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Extensions {
    #[serde(rename = "enabled", default = "extension_enabled")]
    pub enabled: bool,

    #[serde(rename = "dir", default = "extension_dir")]
    pub dir: Option<PathBuf>,

    #[serde(rename = "registries", default)]
    pub registries: Vec<Registry>,
}

impl Extensions {
    pub fn resolve_working_dir<P: AsRef<Path>>(&mut self, dir: P) {
        self.dir = self.dir.take().map(|f| {
            if f.is_relative() {
                PathBuf::from(dir.as_ref()).join(f)
            } else {
                f
            }
        });
    }
}

impl Default for Extensions {
    fn default() -> Self {
        Self {
            enabled: extension_enabled(),
            dir: extension_dir(),
            registries: Default::default(),
        }
    }
}

/// extension_dir is the default value for extensions. This is going to be the
/// place where we store the extensions and load them from.
fn extension_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "kla", "kla")
        .map(|dir| PathBuf::from(dir.config_dir()).join(".extensions"))
}

// extension_enabled is the default value for if the extension simple is enabled
// or not.
fn extension_enabled() -> bool {
    return true;
}

/// Register is used to configure authentication for connecting to a specific
/// registry.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Registry {
    #[serde(rename = "prefix")]
    pub registry: String,
    #[serde(rename = "authentication")]
    pub authentication: Option<Authentication>,
}
