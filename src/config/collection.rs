use std::{cell::RefCell, fs, ops::Deref, path::Path, rc::Rc};

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
/// CollectionsGroups is a Vec of CollectionGroup
pub struct CollectionGroups(Vec<CollectionGroup>);

impl Deref for CollectionGroups {
    type Target = Vec<CollectionGroup>;

    /// return the underlying Vec<CollectionGroups>
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl CollectionGroups {
    /// templates returns an iterator, which returns an iterator of templates which can
    /// safely be executed in parallel. This double iterator system creates a boundry which
    /// clearly defines where execution should fully complete before moving onto the
    /// next set. This ensures an "order of execution" where each environments templates
    /// are executed sequentially, where the whole can be executed with some concurrency
    pub fn templates<'a, L: EnvironmentLoader<Specified> + Copy>(
        &'a self,
        env_loader: L,
    ) -> CollectedTemplateGroup<'a, L> {
        let caching_loader = CachingLoader::new(env_loader);

        CollectedTemplateGroup {
            index: 0,
            env_loader: Rc::new(caching_loader),
            groups: self,
        }
    }
}

/// CollectedTemplateGroup is an iterator that returns an iterator of CollectedEnvironmentGroup
/// Each CollectedEnvironmentGroup returned represents a layer of templates that can be
/// executed concurrently, which will not conflict with ensuring http requests across environments
/// are executed concurrently. You should not attempt to execute from two CollectedEnvironmentGroups
/// concurrently, one should be entirely exhausted before moving onto the next.
pub struct CollectedTemplateGroup<'a, L: EnvironmentLoader<Specified> + Copy> {
    index: usize,
    env_loader: Rc<CachingLoader<Specified, L>>,
    groups: &'a CollectionGroups,
}

impl<'a, L: EnvironmentLoader<Specified> + Copy> Iterator for CollectedTemplateGroup<'a, L> {
    type Item = CollectedEnvironmentGroup<'a, L>;

    /// next returns the next layer of templates. The iterator returned should be fully
    /// exhausted before moving onto the next. Do not run concurrect requests across
    /// multiple CollectedEnvironmentGroups
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.groups.len() {
            return None;
        }

        let group = CollectedEnvironmentGroup {
            depth: self.index,
            index: 0,
            active: None,
            env_loader: Rc::clone(&self.env_loader),
            groups: self.groups,
        };
        self.index += 1;
        Some(group)
    }
}

/// CollectedEnvironmentGroup is an iterator that returns an iterator with all
/// the templates at a specific template depth across all groups. That is, when
/// configured with a depth of 1 will traverse all groups, and return all templates
/// at index 1 for each configured environment. This helps ensure that http requests
/// per environment will be run sequentially while enabling concurrent execution
pub struct CollectedEnvironmentGroup<'a, L: EnvironmentLoader<Specified> + Copy> {
    // depth specifies the template depth, this should never change
    depth: usize,
    // index specifies the index of group we are in, this should increment while
    // iterating
    index: usize,
    // holds onto the current environmentgroup iterator
    active: Option<EnvironmentGroup<'a, L>>,
    env_loader: Rc<CachingLoader<Specified, L>>,
    groups: &'a CollectionGroups,
}

impl<'a, L: EnvironmentLoader<Specified> + Copy> Iterator for CollectedEnvironmentGroup<'a, L> {
    type Item = Result<Template>;

    fn next(&mut self) -> Option<Self::Item> {
        let group = self.groups.get(self.index)?;

        // take the active iterator, if it is some it must have something
        // left to give, if it is empty we should attempt to grab the next
        // groups iterator
        let mut active = if let Some(active) = self.active.take() {
            active
        } else {
            let active = EnvironmentGroup {
                depth: self.depth,
                index: 0,
                env_loader: Rc::clone(&self.env_loader),
                group: group,
            };
            // if we are here we grabbed a new Environment Group so we
            // should increment the index to stage the next environment
            // group when this one is exhausted
            self.index += 1;
            active
        };

        // here we check the value coming out of the active iterator
        // if it has a value we will keep it for the next call and return
        // it's value, if it doesn't have a value we move onto the next
        // iterator by leaving active empty an calling again.
        match active.next() {
            Some(v) => {
                self.active = Some(active);
                Some(v)
            }
            None => self.next(),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct CollectionGroup {
    #[serde(rename = "environments")]
    environments: Vec<String>,
    #[serde(rename = "template", default)]
    templates: Vec<TemplateArgs>,
}

/// Template group is an iterator that returns an iterator of environment groups
/// During execution, the template for each environment should run before the
/// the next template to ensure execution order.
pub struct TemplateGroup<'a, L: EnvironmentLoader<Specified> + Copy> {
    index: usize,
    env_loader: Rc<CachingLoader<Specified, L>>,
    group: &'a CollectionGroup,
}

impl<'a, L: EnvironmentLoader<Specified> + Copy> Iterator for TemplateGroup<'a, L> {
    type Item = EnvironmentGroup<'a, L>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.group.templates.len() {
            return None;
        }

        let group = EnvironmentGroup {
            depth: self.index,
            index: 0,
            env_loader: Rc::clone(&self.env_loader),
            group: self.group,
        };
        self.index += 1;
        Some(group)
    }
}

/// EnvironmentGroup is an iterator that walks through a defined "depth"
/// of the templates. In other words it will return the same template for
/// each environment defined in the group.
pub struct EnvironmentGroup<'a, L: EnvironmentLoader<Specified> + Copy> {
    depth: usize,
    index: usize,
    env_loader: Rc<RefCell<CachingLoader<Specified, L>>>,
    group: &'a CollectionGroup,
    // TODO:
    // add priority from the command line arguments
}

impl<'a, L: EnvironmentLoader<Specified> + Copy> Iterator for EnvironmentGroup<'a, L> {
    type Item = Result<Template>;

    /// Next returns the next environment with the specified iterator
    fn next(&mut self) -> Option<Self::Item> {
        let env = self.group.environments.get(self.index)?;
        self.index += 1;

        let mut env_loader = self.env_loader.clone();
        let env = match env_loader.borrow_mut().load_environment(env) {
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
    groups: CollectionGroups,
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

    /// templates returns an iterator, which returns an iterator of templates which can
    /// safely be executed in parallel. This double iterator system creates a boundry which
    /// clearly defines where execution should fully complete before moving onto the
    /// next set. This ensures an "order of execution" where each environments templates
    /// are executed sequentially, where the whole can be executed with some concurrency
    pub fn templates<'a, L: EnvironmentLoader<Specified> + Copy>(
        &'a self,
        env_loader: L,
    ) -> CollectedTemplateGroup<'a, L> {
        self.groups.templates(env_loader)
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
