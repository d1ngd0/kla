use std::{
    borrow::Cow,
    fmt::{Display, Write},
};

use config::{Config, ConfigError, Value};
use http::Method;
use reqwest::{Client, ClientBuilder, IntoUrl};
use serde::Deserialize;
use skim::SkimItem;

use crate::{Environment, OptBaseURLBuilder, Result, SigV4, URLBuilder};

#[derive(Deserialize, Debug)]
/// Endpoint is a configured environment that specifies a prefix, name, template_dir
/// etc. This struct is the configuration for the final environment that we build
pub struct Endpoint {
    #[serde(rename = "name")]
    /// The name of the environment, used when selecting etc.
    pub name: String,

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

impl Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:", self.name)?;

        if let Some(prefix) = self.prefix.as_ref() {
            write!(f, "[{}]", prefix)?;
        }

        write!(f, "\n")?;

        if let Some(short_description) = self.short_description.as_ref() {
            write!(f, "\t{}\n", short_description)?;
        }

        Ok(())
    }
}

impl SkimItem for Endpoint {
    fn text(&self) -> Cow<'_, str> {
        Cow::from(&self.name)
    }

    fn preview(&self, _context: skim::PreviewContext) -> skim::ItemPreview {
        let mut f = String::new();
        write!(f, "{}:", self.name).unwrap();

        if let Some(prefix) = self.prefix.as_ref() {
            write!(f, "[{}]\n", prefix).unwrap();
        }

        write!(f, "\n").unwrap();

        if let Some(long_description) = self.long_description.as_ref() {
            write!(f, "\n{}\n", long_description).unwrap();
        }
        skim::ItemPreview::Text(f)
    }
}

// Implement try from so we can pass a config directly into the Specified::new
// constructor
impl TryFrom<Config> for Endpoint {
    type Error = ConfigError;

    fn try_from(value: Config) -> std::result::Result<Self, Self::Error> {
        value.try_deserialize()
    }
}

// Implement try from so we can pass a config directly into the Specified::new
// constructor
impl TryFrom<Value> for Endpoint {
    type Error = ConfigError;

    fn try_from(value: Value) -> std::result::Result<Self, Self::Error> {
        value.try_deserialize()
    }
}

/// Specified is an environment type which is specified through the config, it
pub struct Specified {
    name: String,
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

    /// new_with_priority creates a new environment where you get a hook to modify the
    /// clientbuilder and inject your own settings.
    pub async fn new_with_priority<E, C, F>(config: C, overrides: F) -> Result<Self>
    where
        E: Into<crate::Error>,
        C: TryInto<Endpoint, Error = E>,
        F: FnOnce(ClientBuilder) -> Result<ClientBuilder>,
    {
        let mut config: Endpoint = config.try_into().map_err(E::into)?;
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

        // clean up the endpoint
        config.prefix = config.prefix.map(|mut v| {
            if !v.ends_with("/") {
                v.push_str("/");
            }
            v
        });

        Ok(Self {
            name: config.name,
            client: b.build()?,
            url_builder: OptBaseURLBuilder::some_new(config.prefix),
            tmpl_dir: config.template_dir,
            aws_sigv4,
        })
    }
}

impl Environment for Specified {
    fn request<E, M, U>(&self, method: M, url: U) -> Result<reqwest::RequestBuilder>
    where
        E: Into<crate::Error>,
        M: TryInto<Method, Error = E>,
        U: IntoUrl,
    {
        let method = method.try_into().map_err(E::into)?;
        let url = self.url_builder.build(url.as_str())?;
        let b = self.client.request(method, url);
        Ok(b)
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn template_dir(&self) -> Option<&String> {
        self.tmpl_dir.as_ref()
    }

    fn sign(&self, req: reqwest::Request) -> Result<reqwest::Request> {
        if let Some(signer) = self.aws_sigv4.as_ref() {
            Ok(signer.sign(req)?)
        } else {
            Ok(req)
        }
    }

    async fn execute(&self, request: reqwest::Request) -> Result<reqwest::Response> {
        self.client
            .execute(request)
            .await
            .map_err(reqwest::Error::into)
    }
}
