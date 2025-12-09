use std::{collections::HashMap, future::Future, ops::Deref, sync::Arc};

use reqwest::Response;

use crate::{Environment, EnvironmentLoader, Result};

#[derive(Clone, Debug)]
/// CachingLoader will load environments but cache their clients for future use
/// any additional calls to grab an environment will fetch the previously created one.
/// This means any `overridden` client building steps will be ignored on susiquent
/// calls to the caching loader
pub struct CachingLoader<E, L>
where
    E: Environment,
    L: EnvironmentLoader<E>,
{
    env_loader: L,
    envs: HashMap<String, CachedEnvironment<E>>,
}

impl<E: Environment, L: EnvironmentLoader<E>> CachingLoader<E, L> {
    /// creates a new EnvironmentLoader
    pub fn new(loader: L) -> Self {
        CachingLoader {
            env_loader: loader,
            envs: HashMap::new(),
        }
    }
}

impl<E: Environment, L: EnvironmentLoader<E> + Copy> EnvironmentLoader<CachedEnvironment<E>>
    for &mut CachingLoader<E, L>
{
    /// load_environment_with_priority will load the environment and store it into a hashmap
    /// for later retrieval. On each call it will check the hashmap first and return any value
    /// found. IT ONLY CHECKS ON env NAME. Which means any overrides specified will only work
    /// on the first call.
    async fn async_load_environment_with_priority<S, F>(
        self,
        name: S,
        overrides: F,
    ) -> crate::Result<CachedEnvironment<E>>
    where
        S: AsRef<str>,
        F: FnOnce(reqwest::ClientBuilder) -> crate::Result<reqwest::ClientBuilder>,
    {
        let name = name.as_ref();
        match self.envs.get(name) {
            Some(env) => Ok(env.clone()),
            None => {
                let env = self
                    .env_loader
                    .async_load_environment_with_priority(name, overrides)
                    .await?;

                let env = CachedEnvironment::new(env);
                let ret_env = env.clone();
                self.envs.insert(name.into(), env);
                Ok(ret_env)
            }
        }
    }
}

impl<E: Environment, L: EnvironmentLoader<E>> EnvironmentLoader<CachedEnvironment<E>>
    for CachingLoader<E, L>
{
    /// load_environment_with_priority will load the environment and return it as a
    /// cachedEnvironment, however since this impelemtation of the call consumes the
    /// loader we don't actually cache anything
    async fn async_load_environment_with_priority<S, F>(
        self,
        name: S,
        overrides: F,
    ) -> crate::Result<CachedEnvironment<E>>
    where
        S: AsRef<str>,
        F: FnOnce(reqwest::ClientBuilder) -> crate::Result<reqwest::ClientBuilder>,
    {
        let env = self
            .env_loader
            .async_load_environment_with_priority(name, overrides)
            .await?;
        Ok(CachedEnvironment::new(env))
    }
}

#[derive(Debug)]
/// CachedEnvironment is one that has been cached by the CachingLoader, which enables us
/// to return the underlying environment under an Arc
pub struct CachedEnvironment<E: Environment> {
    env: Arc<E>,
}

/// Clone implemented to clone the arc, not the whole client
impl<E: Environment> Clone for CachedEnvironment<E> {
    fn clone(&self) -> Self {
        Self {
            env: Arc::clone(&self.env),
        }
    }
}

impl<E: Environment> CachedEnvironment<E> {
    /// new creates a new cached environment from an actual one
    fn new(env: E) -> Self {
        CachedEnvironment { env: Arc::new(env) }
    }
}

impl<E: Environment> Deref for CachedEnvironment<E> {
    type Target = Arc<E>;

    /// deref returns an Arc of the underlying environment
    fn deref(&self) -> &Self::Target {
        &self.env
    }
}

/// implementing the environment trait means this is an environment and can be used
/// as such
impl<T: Environment> Environment for CachedEnvironment<T> {
    fn request<E, M, U>(&self, method: M, url: U) -> Result<reqwest::RequestBuilder>
    where
        E: Into<crate::Error>,
        M: TryInto<http::Method, Error = E>,
        U: reqwest::IntoUrl,
    {
        self.env.request(method, url)
    }

    fn execute(&self, request: reqwest::Request) -> impl Future<Output = Result<Response>> {
        self.env.execute(request)
    }

    fn name(&self) -> &String {
        self.env.name()
    }
}
