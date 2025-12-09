use crate::{EnvironmentLoader, Error, Expand, Result, Specified};
use anyhow::Context;
use serde::Deserialize;
use std::ffi::OsString;
use std::fs::DirEntry;
use std::path::PathBuf;
use std::{fs, iter, path::Path};

mod command;
pub use command::ConfigCommand;
pub use command::FilterWhen;

mod request;
pub use request::Endpoint;

mod client;
pub use client::*;

mod collection;
pub use collection::*;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    #[serde(rename = "default_environment")]
    pub default_environment: Option<String>,

    #[serde(rename = "settings")]
    pub default_client: Option<Attributes>,

    #[serde(rename = "config", default)]
    pub sub_configs: Vec<SubConfig>,

    #[serde(rename = "environment", default)]
    pub environment: Vec<Endpoint>,

    #[serde(rename = "collection")]
    pub collection_dir: Option<String>,
}

impl Config {
    pub fn from_path<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut config = fs::read_to_string(path.as_ref())
            .with_context(|| format!("could not read file {:?}", path.as_ref()))
            .and_then(|content| {
                toml::from_str::<Config>(&content).with_context(|| format!("could not parse toml"))
            })?;

        let sub_configs = config
            .sub_configs
            .clone()
            .into_iter()
            .map(|v| v.to_configs())
            .flatten();

        for sub_config in sub_configs {
            config = config.merge(sub_config?)
        }

        Ok(config)
    }

    pub fn from_list<S, I>(list: I) -> Result<Self>
    where
        S: AsRef<str>,
        I: Iterator<Item = S>,
    {
        let mut config = list
            .map(|s| s.as_ref().shell_expansion())
            .filter(|f| Path::new(f).exists())
            .next()
            .ok_or(anyhow::Error::msg("No valid config file found"))
            .and_then(|filename| {
                Self::from_path(filename.as_str())
                    .with_context(|| format!("could not read config file {}", filename.as_str()))
            })?;
        config.finalize();

        Ok(config)
    }

    pub fn merge(mut self, other: Self) -> Self {
        let mut other = other;
        // This should not merge, we are intentionally leaving this out, this can only
        // be specified in the root document
        // self.default_environment
        self.environment.append(&mut other.environment);
        self
    }

    pub fn environments(&self) -> std::slice::Iter<'_, Endpoint> {
        (&self.environment).into_iter()
    }

    fn finalize(&mut self) {
        self.default_environment = self
            .default_environment
            .as_ref()
            .map(<&String>::shell_expansion);
    }

    // collection_path is given the name of a collection and renders the path for it.
    // The function just appends the name to the collection directory. If the extension,
    // `.toml`, is missing that is appeneded as well.
    pub fn collection_path(&self, name: &str) -> Result<PathBuf> {
        let name = if name.ends_with(".toml") {
            name.into()
        } else {
            let mut name = String::from(name);
            name.push_str(".toml");
            name
        };

        // create the path
        let mut path = PathBuf::from(
            self.collection_dir
                .as_ref()
                .ok_or_else(|| Error::from(format!("no configured collection directory",)))?,
        );
        path.push(name);

        Ok(path)
    }

    /// collections iterates over the collection_dir and returns each path it finds
    pub fn collections(&self) -> Result<Box<dyn Iterator<Item = String>>> {
        let collection_dir = match self.collection_dir.as_ref() {
            Some(collection) => collection,
            None => return Ok(Box::new(std::iter::empty())),
        };

        let collections = fs::read_dir(collection_dir)?
            .collect::<std::result::Result<Vec<DirEntry>, std::io::Error>>()?
            .into_iter()
            .filter(|f| f.file_type().map(|v| v.is_file()).unwrap_or(false))
            .filter_map(|f| OsString::from(f.path().file_stem()?).into_string().ok());

        Ok(Box::new(collections))
    }
}

impl EnvironmentLoader<Specified> for &Config {
    /// load_environment_with_priority will load a Specified environment from the configuration
    /// and overrides provided.
    async fn async_load_environment_with_priority<S, F>(
        self,
        env: S,
        overrides: F,
    ) -> Result<Specified>
    where
        S: AsRef<str>,
        F: FnOnce(reqwest::ClientBuilder) -> Result<reqwest::ClientBuilder>,
    {
        Specified::from_config_with_priority(env, self, overrides).await
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum SubConfig {
    #[serde(rename = "file")]
    File { path: String },
    #[serde(rename = "dir")]
    Dir { path: String },
}

impl SubConfig {
    fn to_configs(&self) -> Box<dyn Iterator<Item = Result<Config>>> {
        match self {
            SubConfig::File { path } => {
                Box::new(iter::once(Config::from_path(path.shell_expansion())))
            }
            SubConfig::Dir { path } => {
                let entries = match fs::read_dir(path.shell_expansion())
                    .with_context(|| format!("could not read directory {}", path))
                {
                    Ok(entries) => entries,
                    Err(err) => return Box::new(iter::once(Err(err.into()))),
                };

                let entries = entries
                    .map(|entry| entry.map_err(crate::Error::from))
                    .filter(|f| match f.as_ref() {
                        Ok(entry) => entry.file_type().map(|t| t.is_file()).unwrap_or(false),
                        Err(_) => true,
                    })
                    .map(|entry| entry.and_then(|entry| Config::from_path(entry.path())));

                Box::new(entries)
            }
        }
    }
}
