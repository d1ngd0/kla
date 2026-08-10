use std::path::{Path, PathBuf};

use http::Method;
use log::debug;
use reqwest::{Client, ClientBuilder, IntoUrl};
use tera::Value;

use crate::{
    config::{self, Config, Endpoint},
    Attributes, Environment, Error, KResult, OptBaseURLBuilder, URLBuilder, WithAttributes,
};

#[derive(Debug, Clone)]
/// Specified is an environment type which is specified through the config, it
pub struct Specified {
    name: String,
    client: Client,
    url_builder: OptBaseURLBuilder,
    attr: Attributes,
    tmpl_dir: Option<PathBuf>,
    context: Value,
}

impl Specified {
    /// Create a new Specified client from the provided config. The Config is expected
    /// to be deserialized into a
    pub async fn new(config: &Endpoint, attrs: config::Attributes) -> KResult<Self> {
        Self::new_with_priority(config, attrs).await
    }

    /// from_config is passed a path to the environment, and the full configuration
    /// where it will go searching for it and return the associated environment or an error.
    /// if the environment does not exist we return an error
    /// the function also allows specifying overrides
    /// This function also applies the client configurations for the config object itself
    /// so you don't need to do that
    pub async fn from_config_with_priority<S>(
        name: S,
        config: &Config,
        attrs: config::Attributes,
    ) -> KResult<Self>
    where
        S: AsRef<str>,
    {
        debug!("loading environment {} from config", name.as_ref());

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

        Ok(Specified::new_with_priority(endpoint, attrs).await?)
    }

    /// new_with_priority creates a new environment where you get a hook to modify the
    /// clientbuilder and inject your own settings.
    pub async fn new_with_priority(config: &Endpoint, attrs: config::Attributes) -> KResult<Self> {
        debug!(
            "creating new environment from specified config {:#?}",
            &config
        );

        // Clone with configuration from the endpoint, prioritizing the upstream
        // configurations
        let attrs: Attributes = attrs.merge(config.attr.as_ref()).try_into()?;
        let builder = ClientBuilder::new().with_attributes(&attrs)?;

        // clean up the endpoint
        let prefix = config.prefix.as_ref().map(|v| {
            let mut v = v.clone();
            if !v.ends_with("/") {
                v.push_str("/");
            }
            v
        });

        Ok(Self {
            name: config.name.clone(),
            attr: attrs,
            context: config.context.clone(),
            client: builder.build()?,
            url_builder: OptBaseURLBuilder::some_new(prefix),
            tmpl_dir: config.template_dir.clone(),
        })
    }
}

impl Environment for Specified {
    fn request<E, M, U>(&self, method: M, url: U) -> KResult<reqwest::RequestBuilder>
    where
        E: Into<crate::Error>,
        M: TryInto<Method, Error = E>,
        U: IntoUrl,
    {
        let method = method.try_into().map_err(E::into)?;
        let url = self.url_builder.build(url.as_str())?;
        self.client
            .request(method, url)
            .with_attributes(self.attr.as_ref())
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn template_dir(&self) -> Option<&Path> {
        self.tmpl_dir.as_ref().map(|f| f.as_path())
    }

    fn context(&self, context: tera::Context) -> KResult<tera::Context> {
        let mut context = context;
        context.insert("__env", &self.context);
        Ok(context)
    }

    async fn execute(&self, request: reqwest::Request) -> KResult<reqwest::Response> {
        self.client
            .execute(request.with_attributes(&self.attr)?)
            .await
            .map_err(reqwest::Error::into)
    }
}
