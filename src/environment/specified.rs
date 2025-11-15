use http::Method;
use reqwest::{Client, ClientBuilder, IntoUrl};

use crate::{config::Endpoint, Environment, Expand, OptBaseURLBuilder, Result, SigV4, URLBuilder};

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
    pub async fn new(config: &Endpoint) -> Result<Self> {
        Self::new_with_priority(config, |c| Ok(c)).await
    }

    /// new_with_priority creates a new environment where you get a hook to modify the
    /// clientbuilder and inject your own settings.
    pub async fn new_with_priority<F>(config: &Endpoint, overrides: F) -> Result<Self>
    where
        F: FnOnce(ClientBuilder) -> Result<ClientBuilder>,
    {
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
        let prefix = config.prefix.as_ref().map(|v| {
            let mut v = v.clone();
            if !v.ends_with("/") {
                v.push_str("/");
            }
            v
        });

        Ok(Self {
            name: config.name.clone(),
            client: b.build()?,
            url_builder: OptBaseURLBuilder::some_new(prefix),
            tmpl_dir: config.template_dir.as_ref().map(<&String>::shell_expansion),
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
