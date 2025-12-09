use std::{fs, path::Path, rc::Rc};

use crate::{
    config::command::ConfigArgCollection, CachingLoader, ConfigCommand, Environment,
    EnvironmentLoader, Ok as _, Opt as _, Result, Specified, Template, TemplateBuilder,
};
use anyhow::{anyhow, Context as _};
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

pub struct EnvironmentGroup<'a, L: EnvironmentLoader<Specified> + Copy> {
    depth: usize,
    index: usize,
    env_loader: Rc<CachingLoader<Specified, L>>,
    group: &'a CollectionGroup,
}

impl<'a, L: EnvironmentLoader<Specified> + Copy> Iterator for EnvironmentGroup<'a, L> {
    type Item = Result<Template>;

    fn next(&mut self) -> Option<Self::Item> {
        let env = self.group.environments.get(self.index)?;

        let loader = Rc::get_mut(&mut self.env_loader)
            .expect("multiple mutable accesses of environment loader");
        let env = match loader.load_environment(env) {
            Ok(env) => env,
            Err(err) => return Some(Err(err)),
        };

        let tmpl_args = self.group.templates.get(self.depth)?;
        Some(tmpl_args.template(env))
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct TemplateArgs {
    #[serde(rename = "name")]
    name: String,
    #[serde(rename = "args")]
    args: Vec<String>,
}

impl TemplateArgs {
    /// template will turn the TemplateArgs into a Template when given
    /// the environment it should be executing from.
    pub fn template<E: Environment>(&self, env: E) -> Result<Template> {
        let tmpl_config = match ConfigCommand::from_file(env.tmpl_path(&self.name)?.as_path()) {
            Ok(tmpl_config) => tmpl_config,
            Err(_) => {
                return Err(anyhow!("env \"{}\" has no template {}", env.name(), &self.name).into())
            }
        };
        debug!("collection config loaded {:#?}", tmpl_config);

        TemplateBuilder::new().config(tmpl_config).build()
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct CollectionConfig {
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

impl CollectionConfig {
    /// from_file creates a new Collection from a file specified
    pub fn from_file<P>(path: P) -> Result<CollectionConfig>
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
    pub fn with_name<S, C>(name: S, conf: C) -> Result<CollectionConfig>
    where
        S: Into<String>,
        C: AsRef<str>,
    {
        let mut conf: CollectionConfig = toml::from_str(conf.as_ref())?;
        conf.name = name.into();
        Ok(conf)
    }

    // args_context returns a Tera Context object from the arguments specifified
    pub fn args_context(&self, args: &ArgMatches) -> crate::Result<Context> {
        self.args.args_context(args)
    }
}

/// Enable a Collection to be a Command
impl TryFrom<CollectionConfig> for Command {
    type Error = crate::Error;

    fn try_from(value: CollectionConfig) -> std::result::Result<Self, Self::Error> {
        let command = command!()
            .name(&value.name)
            .with_some(value.short_description.as_ref(), Command::about)
            .with_some(value.description.as_ref(), Command::long_about)
            .with_ok_value(<Vec<Arg>>::try_from(value.args), Command::args)
            .with_context(|| format!("{} invalid command configuration", &value.name))?;

        Ok(command)
    }
}
