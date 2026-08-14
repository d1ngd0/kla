use crate::{EnvironmentLoader, Error, Expand, KResult, Specified};
use anyhow::Context;
use log::info;
use serde::Deserialize;
use std::ffi::OsString;
use std::fs::DirEntry;
use std::path::{absolute, PathBuf};
use std::{fs, path::Path};

use super::{Attributes, Endpoint, Extensions, SubConfig};

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    #[serde(rename = "default_environment")]
    pub default_environment: Option<PathBuf>,

    #[serde(rename = "settings", default)]
    pub default_client: Attributes,

    #[serde(rename = "config", default)]
    pub sub_configs: Vec<SubConfig>,

    #[serde(rename = "extensions", default)]
    pub extensions: Extensions,

    #[serde(rename = "environment", default)]
    pub environment: Vec<Endpoint>,

    #[serde(rename = "collection")]
    pub collection_dir: Option<PathBuf>,
}

impl EnvironmentLoader<Specified> for &Config {
    /// load_environment_with_priority will load a Specified environment from the configuration
    /// and overrides provided.
    async fn async_load_environment_with_priority<S>(
        self,
        env: S,
        attrs: Attributes,
    ) -> KResult<Specified>
    where
        S: AsRef<str>,
    {
        Specified::from_config_with_priority(env, self, attrs).await
    }
}

impl Config {
    pub fn from_path<P>(path: P) -> KResult<Self>
    where
        P: AsRef<Path>,
    {
        let dir = absolute(path.as_ref())?;
        let dir = dir
            .parent()
            .with_context(|| format!("could not derive absolute path of {:?}", path.as_ref()))?;

        let mut config = fs::read_to_string(path.as_ref())
            .with_context(|| format!("could not read file {:?}", path.as_ref()))
            .and_then(|content| {
                toml::from_str::<Config>(&content)
                    .with_context(|| format!("could not parse toml from {:?}", path.as_ref()))
            })?;
        config.resolve_working_dir(dir);

        let sub_configs = config
            .sub_configs
            .clone()
            .into_iter()
            .map(|v| v.to_configs(dir))
            .flatten();

        for sub_config in sub_configs {
            config.merge(sub_config?);
        }

        // TODO: Here we will look for the extension file at $extension_dir/extensions.toml and
        // merge the config.toml file from every extension

        Ok(config)
    }

    /// resolve_working_dir finds any relative paths referenced in the config
    /// and resolves them with `dir` as it's base.
    pub fn resolve_working_dir<P: AsRef<Path>>(&mut self, dir: P) {
        let dir = dir.as_ref();
        // Go through all the environments to resolve their working directories
        for env in &mut self.environment {
            env.resolve_working_dir(dir);
        }

        // default environment is also a path, and must have it's relative links made into
        // absolute links
        self.default_environment = self.default_environment.take().map(|f| {
            if f.is_relative() {
                PathBuf::from(dir).join(f)
            } else {
                f
            }
        });

        // Finally do the same for collections
        self.collection_dir = self.collection_dir.take().map(|f| {
            if f.is_relative() {
                PathBuf::from(dir).join(f)
            } else {
                f
            }
        });

        self.extensions.resolve_working_dir(dir);
        self.default_client.resolve_working_dir(dir);
    }

    pub fn from_list<S, I>(list: I) -> KResult<Self>
    where
        S: AsRef<str>,
        I: Iterator<Item = S>,
    {
        let config = list
            .map(|s| s.as_ref().shell_expansion())
            .filter(|f| Path::new(f).exists())
            .next()
            .inspect(|v| info!("Loading config file {}", v))
            .ok_or(anyhow::Error::msg("No valid config file found"))
            .and_then(|filename| {
                Self::from_path(filename.as_str())
                    .with_context(|| format!("could not read config file {}", filename.as_str()))
            })?;

        Ok(config)
    }

    pub fn merge(&mut self, other: Self) -> &mut Self {
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

    // collection_path is given the name of a collection and renders the path for it.
    // The function just appends the name to the collection directory. If the extension,
    // `.toml`, is missing that is appeneded as well.
    pub fn collection_path(&self, name: &str) -> KResult<PathBuf> {
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
    pub fn collections(&self) -> KResult<Box<dyn Iterator<Item = String>>> {
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
