use std::fmt::Display;

use anyhow::Context;
use config::Config;
use reqwest::ClientBuilder;

use super::Environment;

use crate::environment::specified::Endpoint;
use crate::environment::{specified::Specified, unspecified::Unspecified};
use crate::Result;

/// Optional allows you to either specify, or not specify the environment up front.
/// When specified the specified environment underneath is called, when left unspecified
/// the Optional environment contrives some reasonable settings.
pub enum Optional {
    /// Specified is the specified environment which would exist in a config somewhere
    Specified(Specified),
    /// Unspecified is the default when no environment is specified
    Unspecified(Unspecified),
}

impl Optional {
    /// new takes an optional config, when supplied we run the config through Specified.
    /// when left None we use a default unspecified.
    pub async fn new<E, C>(config: Option<C>) -> Result<Self>
    where
        E: Into<crate::Error>,
        C: TryInto<Endpoint, Error = E>,
    {
        let env = match config {
            Some(config) => Optional::Specified(Specified::new(config).await?),
            None => Optional::Unspecified(Unspecified::new()),
        };
        Ok(env)
    }

    /// new takes an optional config, when supplied we run the config through Specified.
    /// when left None we use a default unspecified. Both paths specify the overrides when
    /// generating the underlying client for the environment
    pub async fn new_with_priority<E, C, F>(config: Option<C>, overrides: F) -> Result<Self>
    where
        E: Into<crate::Error>,
        C: TryInto<Endpoint, Error = E>,
        F: FnOnce(ClientBuilder) -> Result<ClientBuilder>,
    {
        let env = match config {
            Some(config) => {
                Optional::Specified(Specified::new_with_priority(config, overrides).await?)
            }
            None => Optional::Unspecified(Unspecified::new_with_priority(overrides)?),
        };
        Ok(env)
    }

    pub async fn from_config<S, F>(name: Option<S>, config: Config, overrides: F) -> Result<Self>
    where
        S: AsRef<str>,
        F: FnOnce(ClientBuilder) -> Result<ClientBuilder>,
    {
        let env = match name {
            Some(name) => Optional::Specified(
                Specified::new_with_priority(
                    config.(name.as_ref()).with_context(|| {
                        format!("environment {} was invalid or not found!", name.as_ref())
                    })?,
                    overrides,
                )
                .await?,
            ),
            None => Optional::Unspecified(Unspecified::new_with_priority(overrides)?),
        };

        Ok(env)
    }
}

// Here we need to make sure to implement all the members, including the default ones, to make
// sure we catch any custom stuff in the lower implementations
impl Environment for Optional {
    fn request<E, M, U>(&self, method: M, url: U) -> crate::Result<reqwest::RequestBuilder>
    where
        E: Into<crate::Error>,
        M: TryInto<http::Method, Error = E>,
        U: reqwest::IntoUrl,
    {
        match self {
            Optional::Specified(specified) => specified.request(method, url),
            Optional::Unspecified(unspecified) => unspecified.request(method, url),
        }
    }

    async fn execute(&self, request: reqwest::Request) -> Result<reqwest::Response> {
        match self {
            Optional::Specified(specified) => specified.execute(request).await,
            Optional::Unspecified(unspecified) => unspecified.execute(request).await,
        }
    }

    fn name(&self) -> &String {
        match self {
            Optional::Specified(specified) => specified.name(),
            Optional::Unspecified(unspecified) => unspecified.name(),
        }
    }

    fn template_dir(&self) -> Option<&String> {
        match self {
            Optional::Specified(specified) => specified.template_dir(),
            Optional::Unspecified(unspecified) => unspecified.template_dir(),
        }
    }

    fn templates(&self) -> crate::Result<Box<dyn Iterator<Item = String>>> {
        match self {
            Optional::Specified(specified) => specified.templates(),
            Optional::Unspecified(unspecified) => unspecified.templates(),
        }
    }

    fn tmpl_path(&self, name: &str) -> crate::Result<std::path::PathBuf> {
        match self {
            Optional::Specified(specified) => specified.tmpl_path(name),
            Optional::Unspecified(unspecified) => unspecified.tmpl_path(name),
        }
    }

    fn sign(&self, req: reqwest::Request) -> crate::Result<reqwest::Request> {
        match self {
            Optional::Specified(specified) => specified.sign(req),
            Optional::Unspecified(unspecified) => unspecified.sign(req),
        }
    }
}

impl Default for Optional {
    /// default returns an unspecified environment
    fn default() -> Self {
        Optional::Unspecified(Unspecified::default())
    }
}

impl Display for Optional {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Optional::Specified(specified) => specified.fmt(f),
            Optional::Unspecified(unspecified) => unspecified.fmt(f),
        }
    }
}
