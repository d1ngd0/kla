use std::borrow::Cow;
use std::fmt::{Display, Write};
use std::fs;
use std::iter;
use std::path::Path;

use anyhow::Context;

mod command;
pub use command::ConfigCommand;
pub use command::FilterWhen;
use serde::Deserialize;
use skim::SkimItem;

use crate::Expand;
use crate::Result;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    #[serde(rename = "default_environment")]
    pub default_environment: Option<String>,

    #[serde(rename = "config", default)]
    pub sub_configs: Vec<SubConfig>,

    #[serde(rename = "environment", default)]
    pub environment: Vec<Endpoint>,
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

#[derive(Deserialize, Debug, Clone)]
/// Endpoint is a configured environment that specifies a prefix, name, template_dir
/// etc. This struct is the configuration for the final environment that we build
pub struct Endpoint {
    #[serde(rename = "name")]
    /// The name of the environment, used when selecting etc.
    pub name: String,

    #[serde(rename = "url")]
    /// The prefix of the environment is prepended to the user supplied
    /// string
    pub prefix: Option<String>,

    #[serde(rename = "short_description")]
    /// short_description is shown to the user when running kla envs
    pub short_description: Option<String>,

    #[serde(rename = "long_description")]
    /// long_description is shown to the user when using the fuzzy finder
    pub long_description: Option<String>,

    #[serde(rename = "template_dir")]
    /// template_dir is a string location to the directory where the templates
    /// for this environment are stored. If there is no directory this should
    /// return None
    pub template_dir: Option<String>,

    /// All the following are for AWS signing of requests. These options are
    /// applied to the request after it is built, and require usage of the
    /// WithEnvironment trait on the request itself
    #[serde(rename = "sigv4")]
    pub sigv4: Option<bool>,
    #[serde(rename = "sigv4_aws_profile")]
    pub sigv4_aws_profile: Option<String>,
    #[serde(rename = "sigv4_aws_service")]
    pub sigv4_aws_service: Option<String>,
}

impl Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:", self.name)?;

        if let Some(prefix) = self.prefix.as_ref() {
            write!(f, "[{}]", prefix)?;
        }

        write!(f, "\n")?;

        if let Some(short_description) = self.short_description.as_ref() {
            write!(f, "\t{}\n", short_description)?;
        }

        Ok(())
    }
}

impl SkimItem for Endpoint {
    fn text(&self) -> Cow<'_, str> {
        Cow::from(&self.name)
    }

    fn preview(&self, _context: skim::PreviewContext) -> skim::ItemPreview {
        let mut f = String::new();
        write!(f, "{}:", self.name).unwrap();

        if let Some(prefix) = self.prefix.as_ref() {
            write!(f, "[{}]\n", prefix).unwrap();
        }

        write!(f, "\n").unwrap();

        if let Some(long_description) = self.long_description.as_ref() {
            write!(f, "\n{}\n", long_description).unwrap();
        }
        skim::ItemPreview::Text(f)
    }
}
