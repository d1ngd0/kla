use reqwest::{ClientBuilder, Request, RequestBuilder};

use super::Environment;
use crate::Result;

/// WithEnvironment allows multiple builders to take on attributes
/// defined within an environment. With this trait you can simply specify
/// with_environment on the specific builder, and the settings relevant will
/// get applied
pub trait WithEnvironment: Sized {
    fn with_environment<E>(self, env: &E) -> impl std::future::Future<Output = Result<Self>>
    where
        E: Environment;
}

impl WithEnvironment for ClientBuilder {
    /// with_environment will:
    async fn with_environment<E>(self, _env: &E) -> Result<Self>
    where
        E: Environment,
    {
        Ok(self)
    }
}

impl WithEnvironment for RequestBuilder {
    /// with_environment will:
    async fn with_environment<E>(self, _env: &E) -> Result<Self>
    where
        E: Environment,
    {
        Ok(self)
    }
}

impl WithEnvironment for Request {
    /// with_environment will
    /// - Sign the request if there is a configured signature
    async fn with_environment<E>(self, env: &E) -> Result<Self>
    where
        E: Environment,
    {
        // sign the request
        env.sign(self)
    }
}
