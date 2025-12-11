use crate::{CollectedTemplateGroup, CollectionConfig, EnvironmentLoader, Result, Specified};
use anyhow::anyhow;
use clap::ArgMatches;
use reqwest::ClientBuilder;

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

    pub fn build<F>(mut self, _client_builder: F) -> Result<Collection<'a, E>>
    where
        F: Fn(ClientBuilder) -> Result<ClientBuilder>,
    {
        Ok(Collection {
            templates: self
                .config
                .ok_or_else(|| anyhow!("must specify config for collection"))?
                .templates(self.env_loader),
        })
    }
}

pub struct Collection<'a, E: EnvironmentLoader<Specified> + Copy> {
    templates: CollectedTemplateGroup<'a, E>,
}

impl<'a, E: EnvironmentLoader<Specified> + Copy> Collection<'a, E> {
    pub fn run(self, args: &ArgMatches, dry: bool) -> Result<()> {
        dbg!(args, dry);
        for group in self.templates {
            for template in group {
                dbg!(template?);
            }
        }
        Ok(())
    }
}
