use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::Result;
use inquire::{Password, PasswordDisplayMode};
use serde::{Deserialize, Serialize};

/// FileOrValue holds a file with a value in it, or a literal value
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum SecretValue {
    File { path: PathBuf, trim: bool },
    Value(String),
    Prompt { prompt: String },
}

impl AsRef<SecretValue> for SecretValue {
    fn as_ref(&self) -> &SecretValue {
        self
    }
}

impl SecretValue {
    /// resolve_working_dir finds any relative paths referenced in the config
    /// and resolves them with `dir` as it's base.
    pub fn resolve_working_dir<P: AsRef<Path>>(&mut self, dir: P) {
        match self {
            SecretValue::File { path, trim: _ } => {
                if path.is_relative() {
                    *path = PathBuf::from(dir.as_ref()).join(path.as_path());
                }
            }
            _ => (),
        }
    }

    pub fn cached(self) -> CachedSecretValue {
        CachedSecretValue(Arc::new(Mutex::new(Some(self))))
    }

    pub fn to_string(self) -> Result<String> {
        match self {
            SecretValue::File { path, trim } => {
                let s = read_to_string(path)?;
                if trim {
                    Ok(s.trim_end().to_string())
                } else {
                    Ok(s)
                }
            }
            SecretValue::Value(value) => Ok(value),
            SecretValue::Prompt { prompt } => Ok(Password::new(&format!("{}: ", &prompt))
                .without_confirmation()
                .with_display_mode(PasswordDisplayMode::Masked)
                .prompt()?),
        }
    }
}

impl From<String> for SecretValue {
    fn from(value: String) -> Self {
        Self::Value(value)
    }
}

impl From<&str> for SecretValue {
    fn from(value: &str) -> Self {
        Self::from(value.to_string())
    }
}

impl TryFrom<SecretValue> for String {
    type Error = crate::Error;

    /// Consume the ClientSecret and return an oauth::ClientSecret
    fn try_from(value: SecretValue) -> Result<Self> {
        value.to_string()
    }
}

/// CachedSecretValue will store the value after being asked for the first time
/// in memory, so it will only prompt you for the value once. You can pair this with
/// a FileCache to reduce requests across requests.
#[derive(Debug, Clone)]
pub struct CachedSecretValue(Arc<Mutex<Option<SecretValue>>>);

impl CachedSecretValue {
    pub fn new(sv: SecretValue) -> CachedSecretValue {
        CachedSecretValue(Arc::new(Mutex::new(Some(sv))))
    }
}

impl CachedSecretValue {
    pub fn to_string(&self) -> Result<String> {
        let mut lock = self.0.lock().expect("poisoned lock");
        let val = lock
            .take()
            .expect("Option should always have value")
            .to_string()?;
        lock.replace(SecretValue::Value(val.clone()));
        Ok(val)
    }
}

impl From<SecretValue> for CachedSecretValue {
    fn from(value: SecretValue) -> Self {
        CachedSecretValue::new(value)
    }
}

impl From<String> for CachedSecretValue {
    fn from(value: String) -> Self {
        CachedSecretValue::new(SecretValue::Value(value))
    }
}

impl From<&str> for CachedSecretValue {
    fn from(value: &str) -> Self {
        CachedSecretValue::new(SecretValue::Value(value.into()))
    }
}

impl TryFrom<&CachedSecretValue> for String {
    type Error = crate::Error;

    fn try_from(value: &CachedSecretValue) -> Result<Self> {
        value.to_string()
    }
}
