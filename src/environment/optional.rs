use std::fmt::Display;
use std::fmt::Pointer;

use reqwest::ClientBuilder;

use super::Environment;

use crate::config::Config;
use crate::config::Endpoint;
use crate::environment::{specified::Specified, unspecified::Unspecified};
use crate::Opt;
use crate::WithAttributes;
use crate::{Error, Result};

#[derive(Debug, Clone)]
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
    pub async fn new(builder: ClientBuilder, config: Option<&Endpoint>) -> Result<Self> {
        let env = match config {
            Some(config) => Optional::Specified(Specified::new(builder, config).await?),
            None => Optional::Unspecified(Unspecified::new(builder)?),
        };
        Ok(env)
    }

    /// new takes an optional config, when supplied we run the config through Specified.
    /// when left None we use a default unspecified. Both paths specify the overrides when
    /// generating the underlying client for the environment
    pub async fn new_with_priority<F>(
        builder: ClientBuilder,
        config: Option<&Endpoint>,
        overrides: F,
    ) -> Result<Self>
    where
        F: FnOnce(ClientBuilder) -> Result<ClientBuilder>,
    {
        let env = match config {
            Some(config) => {
                Optional::Specified(Specified::new_with_priority(builder, config, overrides).await?)
            }
            None => Optional::Unspecified(Unspecified::new_with_priority(builder, overrides)?),
        };
        Ok(env)
    }

    /// from_config is passed a path to the environment, and the full configuration
    /// where it will go searching for it and return the associated environment or an error
    /// the function also allows specifying overrides
    /// This function also applies the client configurations for the config object itself
    /// so you don't need to do that
    pub async fn from_config_with_priority<S, F>(
        name: Option<S>,
        config: &Config,
        overrides: F,
    ) -> Result<Self>
    where
        S: AsRef<str>,
        F: FnOnce(ClientBuilder) -> Result<ClientBuilder>,
    {
        // here we add the default client configuration, meaning it has the lowest priority
        // for setting the value. If you create an environment without using this function
        // you need to add default_client yourself
        let builder = ClientBuilder::new().with_some_result(
            config.default_client.as_ref(),
            ClientBuilder::with_attributes,
        )?;

        let env = match name {
            Some(name) => {
                let endpoint = config
                    .environments()
                    .filter(|env| env.name == name.as_ref())
                    .next()
                    .ok_or_else(|| {
                        Error::from(format!(
                            "environment {} was invalid or not found!",
                            name.as_ref()
                        ))
                    })?;

                Optional::Specified(
                    Specified::new_with_priority(builder, endpoint, overrides).await?,
                )
            }
            None => Optional::Unspecified(Unspecified::new_with_priority(builder, overrides)?),
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
