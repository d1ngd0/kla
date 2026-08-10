use crate::KResult;
use anyhow::Context;
use serde::Deserialize;
use std::path::PathBuf;
use std::{fs, iter, path::Path};

use super::Config;

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum SubConfig {
    #[serde(rename = "file")]
    File { path: PathBuf },
    #[serde(rename = "dir")]
    Dir { path: PathBuf },
}

impl SubConfig {
    pub fn to_configs<P>(&self, working_dir: P) -> Box<dyn Iterator<Item = KResult<Config>>>
    where
        P: AsRef<Path>,
    {
        match self {
            SubConfig::File { path } => {
                let buf: PathBuf;
                let path = if path.is_relative() {
                    buf = PathBuf::from(working_dir.as_ref()).join(path);
                    &buf
                } else {
                    path
                };

                Box::new(iter::once(Config::from_path(path)))
            }
            SubConfig::Dir { path } => {
                let buf: PathBuf;
                let path = if path.is_relative() {
                    buf = PathBuf::from(working_dir.as_ref()).join(path);
                    &buf
                } else {
                    path
                };

                let entries = match fs::read_dir(path)
                    .with_context(|| format!("could not read directory {:#?}", path))
                {
                    Ok(entries) => entries,
                    Err(err) => return Box::new(iter::once(Err(err.into()))),
                };

                let entries = entries
                    .map(|entry| entry.map_err(crate::Error::from))
                    .filter(|f| match f.as_ref() {
                        // you need to check and make sure it is toml here
                        Ok(entry) => entry.file_type().map(|t| t.is_file()).unwrap_or(false),
                        Err(_) => true,
                    })
                    .map(|entry| entry.and_then(|entry| Config::from_path(entry.path())));

                Box::new(entries)
            }
        }
    }
}
