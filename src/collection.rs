use crate::{CachingLoader, CollectionConfig, EnvironmentLoader, Result, Specified};
use clap::ArgMatches;
use reqwest::ClientBuilder;

#[derive(Debug, Clone)]
pub struct CollectionBuilder<'a, E: EnvironmentLoader<Specified>> {
    env_loader: CachingLoader<Specified, E>,
    config: Option<&'a CollectionConfig>,
}

impl<'a, E: EnvironmentLoader<Specified>> CollectionBuilder<'a, E> {
    pub fn new(environment_loader: E) -> Self {
        let caching_loader = CachingLoader::new(environment_loader);

        Self {
            env_loader: caching_loader,
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
        for self.config.
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

