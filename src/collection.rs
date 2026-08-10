use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::{
    config::CollectedEnvironmentGroup, config::CollectedTemplateGroup, config::CollectionConfig,
    config::ConfigArgCollection, config::ExecutableTemplate, CachedEnvironment, EnvironmentLoader,
    KResult, Output, Specified,
};
use anyhow::anyhow;
use clap::ArgMatches;
use log::error;
use tera::Context;
use tokio::{io::stdout, task::JoinSet};

#[derive(Debug, Clone)]
pub struct CollectionBuilder<'a, E: EnvironmentLoader<Specified> + Copy> {
    env_loader: E,
    config: Option<&'a CollectionConfig>,
}

impl<'a, E: EnvironmentLoader<Specified> + Copy> CollectionBuilder<'a, E> {
    pub fn new(env_loader: E) -> Self {
        Self {
            env_loader,
            config: None,
        }
    }

    pub fn config(mut self, config: &'a CollectionConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn build(self) -> KResult<Collection> {
        let config = self
            .config
            .ok_or_else(|| anyhow!("must specify config for collection"))?;

        Ok(Collection {
            templates: Sets::try_from(config.templates(self.env_loader))?,
            args: config.args.clone(),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Group(Vec<ExecutableTemplate<CachedEnvironment<Specified>>>);

impl<L: EnvironmentLoader<Specified> + Copy> TryFrom<CollectedEnvironmentGroup<'_, L>> for Group {
    type Error = crate::Error;

    fn try_from(value: CollectedEnvironmentGroup<'_, L>) -> std::result::Result<Self, Self::Error> {
        let mut cg = Group::default();
        for template in value {
            cg.push(template?);
        }
        Ok(cg)
    }
}

impl Deref for Group {
    type Target = Vec<ExecutableTemplate<CachedEnvironment<Specified>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Group {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Group {
    fn run(&self, context: &Context, dry: bool) -> Vec<KResult<Output>> {
        let mut join_set = JoinSet::new();
        // GROSS
        let groups = self.0.clone();
        let context = Arc::new(context.clone());

        for tmpl in groups {
            join_set.spawn(tmpl.run(Arc::clone(&context), dry));
        }

        futures::executor::block_on(join_set.join_all())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Sets(Vec<Group>);

impl<L: EnvironmentLoader<Specified> + Copy> TryFrom<CollectedTemplateGroup<'_, L>> for Sets {
    type Error = crate::Error;

    fn try_from(value: CollectedTemplateGroup<'_, L>) -> std::result::Result<Self, Self::Error> {
        let mut c = Sets::default();
        for group in value {
            let group = Group::try_from(group)?;
            c.push(group);
        }
        Ok(c)
    }
}

impl Deref for Sets {
    type Target = Vec<Group>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Sets {
    pub fn run(&self, context: &Context, dry: bool) -> Vec<KResult<Output>> {
        self.iter().map(|v| v.run(context, dry)).flatten().collect()
    }
}

impl DerefMut for Sets {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Debug)]
pub struct Collection {
    templates: Sets,
    args: ConfigArgCollection,
}

impl Collection {
    pub async fn run(self, args: &ArgMatches, dry: bool) -> KResult<()> {
        let context = self.args.args_context(args)?;
        let outputs = self.templates.run(&context, dry);

        for output in outputs {
            let mut out = stdout();
            match output {
                Ok(output) => {
                    output.copy(&mut out).await?;
                }
                Err(err) => error!("request failed: {}", err),
            }
        }
        Ok(())
    }
}
