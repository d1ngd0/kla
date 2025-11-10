use std::time::SystemTime;

use anyhow::Context;
use aws_config::BehaviorVersion;
use aws_credential_types::provider::ProvideCredentials;
use config::{Config, ConfigError};
use reqwest::{Client, ClientBuilder};
use serde::Deserialize;

use crate::{AssumingURLBuilder, Opt, OptBaseURLBuilder, Result, SigV4};

#[derive(Deserialize, Debug)]
/// Endpoint is a configured environment that specifies a prefix, name, template_dir
/// etc. This struct is the configuration for the final environment that we build
pub struct Endpoint {
    #[serde(rename = "name")]
    /// The name of the environment, used when selecting etc.
    name: String,

    #[serde(rename = "url")]
    /// The prefix of the environment is prepended to the user supplied
    /// string
    prefix: Option<String>,

    #[serde(rename = "short_description")]
    /// short_description is shown to the user when running kla envs
    short_description: Option<String>,

    #[serde(rename = "long_description")]
    /// long_description is shown to the user when using the fuzzy finder
    long_description: Option<String>,

    #[serde(rename = "template_dir")]
    /// template_dir is a string location to the directory where the templates
    /// for this environment are stored. If there is no directory this should
    /// return None
    template_dir: Option<String>,

    /// All the following are for AWS signing of requests. These options are
    /// applied to the request after it is built, and require usage of the
    /// WithEnvironment trait on the request itself
    #[serde(rename = "sigv4")]
    sigv4: Option<bool>,
    #[serde(rename = "sigv4_aws_profile")]
    sigv4_aws_profile: Option<String>,
    #[serde(rename = "sigv4_aws_service")]
    sigv4_aws_service: Option<String>,
}

// Implement try from so we can pass a config directly into the Specified::new
// constructor
impl TryFrom<Config> for Endpoint {
    type Error = ConfigError;

    fn try_from(value: Config) -> std::result::Result<Self, Self::Error> {
        value.try_deserialize()
    }
}

/// Specified is an environment type which is specified through the config, it
pub struct Specified {
    client: Client,
    url_builder: OptBaseURLBuilder,
    tmpl_dir: Option<String>,
    aws_sigv4: Option<SigV4>,
}

impl Specified {
    /// Create a new Specified client from the provided config. The Config is expected
    /// to be deserialized into a
    pub async fn new<E, C>(config: C) -> Result<Self>
    where
        E: Into<crate::Error>,
        C: TryInto<Endpoint, Error = E>,
    {
        Self::new_with_priority(config, |c| Ok(c)).await
    }

    pub async fn new_with_priority<E, C, F>(config: C, overrides: F) -> Result<Self>
    where
        E: Into<crate::Error>,
        C: TryInto<Endpoint, Error = E>,
        F: FnOnce(ClientBuilder) -> Result<ClientBuilder>,
    {
        let config: Endpoint = config.try_into().map_err(E::into)?;
        // Add Client level specifications here
        let b = ClientBuilder::new();
        let b = overrides(b)?;

        let mut aws_sigv4: Option<SigV4> = None;
        if config.sigv4.unwrap_or(false) {
            aws_sigv4 = Some(
                SigV4::new(
                    config.sigv4_aws_profile.as_ref(),
                    config.sigv4_aws_service.as_ref(),
                )
                .await?,
            )
        }

        // TODO: you were here, this doesn't work
        config.prefix.map(|v| if !endpoint.prefix.ends_with("/") {
            endpoint.prefix.push_str("/");
        });

        Ok(Self {
            client: b.build()?,
            url_builder: OptBaseURLBuilder::some_new(config.prefix),
            tmpl_dir: config.template_dir,
            aws_sigv4,
        })
    }
}

impl Environment for 
