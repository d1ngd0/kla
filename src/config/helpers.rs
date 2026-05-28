use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

/// FileOrValue holds a file with a value in it, or a literal value
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum FileOrValue {
    File { path: PathBuf, trim: bool },
    Value(String),
}

impl FileOrValue {
    /// resolve_working_dir finds any relative paths referenced in the config
    /// and resolves them with `dir` as it's base.
    pub fn resolve_working_dir<P: AsRef<Path>>(&mut self, dir: P) {
        match self {
            FileOrValue::File { path, trim: _ } => {
                if path.is_relative() {
                    *path = PathBuf::from(dir.as_ref()).join(path.as_path());
                }
            }
            FileOrValue::Value(_) => (),
        }
    }
}

impl TryFrom<FileOrValue> for String {
    type Error = crate::Error;

    /// Consume the ClientSecret and return an oauth::ClientSecret
    fn try_from(value: FileOrValue) -> Result<Self, Self::Error> {
        match value {
            FileOrValue::File { path, trim } => {
                let s = read_to_string(path)?;
                if trim {
                    Ok(s.trim_end().to_string())
                } else {
                    Ok(s)
                }
            }
            FileOrValue::Value(value) => Ok(value),
        }
    }
}
