use crate::{
    CachingLoader, CollectedTemplateGroup, CollectionConfig, EnvironmentLoader, Result, Specified,
};
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

    pub fn build<F>(mut self, _client_builder: F) -> Result<Collection>
    where
        F: Fn(ClientBuilder) -> Result<ClientBuilder>,
    {
        // TODO: you were here, just testing things out
        let groups = self.config.unwrap().templates(self.env_loader);
        for group in groups {
            for template in group {
                dbg!(template?);
            }
        }

        Ok(Collection {})
    }
}

pub struct Collection {}

impl Collection {
    pub fn run(self, args: &ArgMatches, dry: bool) -> Result<()> {
        dbg!(args, dry);
        Ok(())
    }
}
