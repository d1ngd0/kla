use std::{fs, path::Path};

use crate::{config::command::ConfigArgCollection, Ok as _, Opt as _, Result};
use anyhow::Context as _;
use clap::{command, Arg, ArgMatches, Command};
use log::debug;
use serde::Deserialize;
use tera::Context;

#[derive(Deserialize, Debug, Clone)]
pub struct CollectionGroup {
    #[serde(rename = "environments")]
    environments: Vec<String>,
    #[serde(rename = "template", default)]
    templates: Vec<TemplateArgs>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TemplateArgs {
    #[serde(rename = "name")]
    name: String,
    #[serde(rename = "args")]
    args: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Collection {
    #[serde(skip)]
    pub name: String,

    #[serde(rename = "arg", default)]
    args: ConfigArgCollection,

    #[serde(rename = "short_description")]
    short_description: Option<String>,

    #[serde(rename = "description")]
    description: Option<String>,

    #[serde(rename = "group")]
    groups: Vec<CollectionGroup>,
}

impl Collection {
    /// from_file creates a new Collection from a file specified
    pub fn from_file<P>(path: P) -> Result<Collection>
    where
        P: AsRef<Path>,
    {
        let name = path
            .as_ref()
            .file_name()
            .and_then(|filename| filename.to_str())
            .and_then(|filename| filename.strip_suffix(".toml"))
            .ok_or_else(|| {
                crate::Error::from(format!(
                    "could not get collection name from path {:?}",
                    path.as_ref()
                ))
            })?;
        let content = fs::read_to_string(path.as_ref())?;
        debug!("collection \"{}\" with content\n{:?}", name, content);
        Self::with_name(name, content)
    }

    // with name takes a name and a toml configuration string and returns
    // a collection
    pub fn with_name<S, C>(name: S, conf: C) -> Result<Collection>
    where
        S: Into<String>,
        C: AsRef<str>,
    {
        let mut conf: Collection = toml::from_str(conf.as_ref())?;
        conf.name = name.into();
        Ok(conf)
    }

    // args_context returns a Tera Context object from the arguments specifified
    pub fn args_context(&self, args: &ArgMatches) -> crate::Result<Context> {
        self.args.args_context(args)
    }
}

/// Enable a Collection to be a Command
impl TryFrom<Collection> for Command {
    type Error = crate::Error;

    fn try_from(value: Collection) -> std::result::Result<Self, Self::Error> {
        let command = command!()
            .name(&value.name)
            .with_some(value.short_description.as_ref(), Command::about)
            .with_some(value.description.as_ref(), Command::long_about)
            .with_ok_value(<Vec<Arg>>::try_from(value.args), Command::args)
            .with_context(|| format!("{} invalid command configuration", &value.name))?;

        Ok(command)
    }
}
