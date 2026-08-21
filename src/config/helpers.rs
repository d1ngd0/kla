use std::{
    env,
    fs::read_to_string,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::{clap::edit_value, Error, KResult};
use clap::builder::ArgExt;
use inquire::{Password, PasswordDisplayMode, Select, Text};
use serde::{Deserialize, Serialize};

/// FileOrValue holds a file with a value in it, or a literal value
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum ValueSource {
    Value(String),
    File {
        path: PathBuf,
        trim: Option<bool>,
    },
    Text {
        text: String,
    },
    Password {
        password: String,
    },
    Env {
        env: String,
        default: Option<String>,
    },
    Editor {
        editor: String,
    },
    Select {
        select: String,
        items: Vec<String>,
    },
}

impl ArgExt for ValueSource {}

impl AsRef<ValueSource> for ValueSource {
    fn as_ref(&self) -> &ValueSource {
        self
    }
}

impl ValueSource {
    /// resolve_working_dir finds any relative paths referenced in the config
    /// and resolves them with `dir` as it's base.
    pub fn resolve_working_dir<P: AsRef<Path>>(&mut self, dir: P) {
        match self {
            ValueSource::File { path, trim: _ } => {
                if path.is_relative() {
                    *path = PathBuf::from(dir.as_ref()).join(path.as_path());
                }
            }
            _ => (),
        }
    }

    pub fn cached(self) -> CachedValueSource {
        CachedValueSource(Arc::new(Mutex::new(Some(self))))
    }

    pub fn to_string(self) -> KResult<String> {
        Ok(match self {
            ValueSource::File { path, trim } => {
                let s = read_to_string(path)?;
                if trim.unwrap_or(false) {
                    s.trim_end().to_string()
                } else {
                    s
                }
            }
            ValueSource::Value(value) => value,
            Self::Text { text } => Text::new(&text).prompt()?,
            ValueSource::Password { password } => Password::new(&format!("{}: ", &password))
                .without_confirmation()
                .with_display_mode(PasswordDisplayMode::Masked)
                .prompt()?,
            ValueSource::Env { env, default } => env::var(&env)
                .ok()
                .or(default)
                .ok_or_else(|| Error::from(format!("{} is not set", &env)))?,
            ValueSource::Editor { editor } => {
                futures::executor::block_on(edit_value::<String, String>(editor))?
            }
            ValueSource::Select { select, items } => Select::new(&select, items).prompt()?,
        })
    }
}

impl From<String> for ValueSource {
    fn from(value: String) -> Self {
        Self::Value(value)
    }
}

impl From<&str> for ValueSource {
    fn from(value: &str) -> Self {
        Self::from(value.to_string())
    }
}

impl TryFrom<ValueSource> for String {
    type Error = crate::Error;

    /// Consume the ClientSecret and return an oauth::ClientSecret
    fn try_from(value: ValueSource) -> KResult<Self> {
        value.to_string()
    }
}

/// CachedSecretValue will store the value after being asked for the first time
/// in memory, so it will only prompt you for the value once. You can pair this with
/// a FileCache to reduce requests across requests.
#[derive(Debug, Clone)]
pub struct CachedValueSource(Arc<Mutex<Option<ValueSource>>>);

impl CachedValueSource {
    pub fn new(sv: ValueSource) -> CachedValueSource {
        CachedValueSource(Arc::new(Mutex::new(Some(sv))))
    }
}

impl CachedValueSource {
    pub fn to_string(&self) -> KResult<String> {
        let mut lock = self.0.lock().expect("poisoned lock");
        let val = lock
            .take()
            .expect("Option should always have value")
            .to_string()?;
        lock.replace(ValueSource::Value(val.clone()));
        Ok(val)
    }
}

impl From<ValueSource> for CachedValueSource {
    fn from(value: ValueSource) -> Self {
        CachedValueSource::new(value)
    }
}

impl From<String> for CachedValueSource {
    fn from(value: String) -> Self {
        CachedValueSource::new(ValueSource::Value(value))
    }
}

impl From<&str> for CachedValueSource {
    fn from(value: &str) -> Self {
        CachedValueSource::new(ValueSource::Value(value.into()))
    }
}

impl TryFrom<&CachedValueSource> for String {
    type Error = crate::Error;

    fn try_from(value: &CachedValueSource) -> KResult<Self> {
        value.to_string()
    }
}
