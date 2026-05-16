use std::path::{Path, PathBuf};

use http::Method;
use log::debug;
use reqwest::{Client, ClientBuilder, IntoUrl, RequestBuilder};
use tera::Value;

use crate::{
    config::Endpoint, Attributes, Config, Environment, Error, Opt, OptBaseURLBuilder, Result,
    SigV4, URLBuilder, WithAttributes,
};

#[derive(Debug, Clone)]
/// Specified is an environment type which is specified through the config, it
pub struct Specified {
    name: String,
    client: Client,
    url_builder: OptBaseURLBuilder,
    attr: Option<Attributes>,
    tmpl_dir: Option<PathBuf>,
    aws_sigv4: Option<SigV4>,
    context: Value,
}

impl Specified {
    /// Create a new Specified client from the provided config. The Config is expected
    /// to be deserialized into a
    pub async fn new(builder: ClientBuilder, config: &Endpoint) -> Result<Self> {
        Self::new_with_priority(builder, config, |c| Ok(c)).await
    }

    /// from_config is passed a path to the environment, and the full configuration
    /// where it will go searching for it and return the associated environment or an error.
    /// if the environment does not exist we return an error
    /// the function also allows specifying overrides
    /// This function also applies the client configurations for the config object itself
    /// so you don't need to do that
    pub async fn from_config_with_priority<S, F>(
        name: S,
        config: &Config,
        overrides: F,
    ) -> Result<Self>
    where
        S: AsRef<str>,
        F: FnOnce(ClientBuilder) -> Result<ClientBuilder>,
    {
        debug!("loading environment {} from config", name.as_ref());
        // here we add the default client configuration, meaning it has the lowest priority
        // for setting the value. If you create an environment without using this function
        // you need to add default_client yourself
        let builder = ClientBuilder::new().with_some_result(
            config.default_client.as_ref(),
            ClientBuilder::with_attributes,
        )?;

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

        Ok(Specified::new_with_priority(builder, endpoint, overrides).await?)
    }

    /// new_with_priority creates a new environment where you get a hook to modify the
    /// clientbuilder and inject your own settings.
    pub async fn new_with_priority<F>(
        builder: ClientBuilder,
        config: &Endpoint,
        overrides: F,
    ) -> Result<Self>
    where
        F: FnOnce(ClientBuilder) -> Result<ClientBuilder>,
    {
        debug!(
            "creating new environment from specified config {:#?}",
            &config
        );

        // here we add the environment level configurations, along with the overrides, this
        // means that the environment specified attributes can be overloaded by the overrides
        // which is often the cli arguments
        let b = overrides(
            builder.with_some_result(config.attr.as_ref(), ClientBuilder::with_attributes)?,
        )?;

        let mut aws_sigv4: Option<SigV4> = None;
        if let Some(attr) = config.attr.as_ref() {
            if attr.sigv4.unwrap_or(false) {
                aws_sigv4 = Some(
                    SigV4::new(
                        attr.sigv4_aws_profile.as_ref(),
                        attr.sigv4_aws_service.as_ref(),
                    )
                    .await?,
                )
            }
        }

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
            attr: config.attr.clone(),
            context: config.context.clone(),
            client: b.build()?,
            url_builder: OptBaseURLBuilder::some_new(prefix),
            tmpl_dir: config.template_dir.clone(),
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
        let b = self
            .client
            .request(method, url)
            .with_some_result(self.attr.as_ref(), RequestBuilder::with_attributes)?;

        Ok(b)
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn template_dir(&self) -> Option<&Path> {
        self.tmpl_dir.as_ref().map(|f| f.as_path())
    }

    fn sign(&self, req: reqwest::Request) -> Result<reqwest::Request> {
        if let Some(signer) = self.aws_sigv4.as_ref() {
            Ok(signer.sign(req)?)
        } else {
            Ok(req)
        }
    }

    fn context(&self, context: tera::Context) -> Result<tera::Context> {
        let mut context = context;
        context.insert("__env", &self.context);
        Ok(context)
    }

    async fn execute(&self, request: reqwest::Request) -> Result<reqwest::Response> {
        self.client
            .execute(request)
            .await
            .map_err(reqwest::Error::into)
    }
}
