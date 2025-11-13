use std::fmt::Display;

use reqwest::{Client, ClientBuilder};

use crate::{Environment, Result};

#[derive(Clone, Debug)]
/// Unspecified is an environment which does not have any configured
/// context. It does it's best to render the users query with no environment
/// specified
pub struct Unspecified {
    name: String,
    client: Client,
}

impl Unspecified {
    /// new is just shorthand for Self::default(). This creates a default
    /// reqwest::Client for making the requests, and a name of "default"
    pub fn new() -> Self {
        Self::default()
    }

    /// new_with_priority creates an unspecified environment where you get to
    /// alter the client build attributes.
    pub fn new_with_priority<F>(overrides: F) -> Result<Self>
    where
        F: FnOnce(ClientBuilder) -> Result<ClientBuilder>,
    {
        let b = ClientBuilder::new();
        let b = overrides(b)?;

        Ok(Self {
            name: String::from("default"),
            client: b.build()?,
        })
    }
}

impl Default for Unspecified {
    /// default will build the default implementation of the UnspecifiedClient
    /// It will capture a default reqwest::Client with "default" for the name
    fn default() -> Self {
        Self {
            name: String::from("default"),
            client: Default::default(),
        }
    }
}

impl Environment for Unspecified {
    /// request assumes the url is a fully qualified url, nothing is done to change
    /// that. A request builder is returned from the internal client
    fn request<E, M, U>(&self, method: M, url: U) -> Result<reqwest::RequestBuilder>
    where
        E: Into<crate::Error>,
        M: TryInto<http::Method, Error = E>,
        U: reqwest::IntoUrl,
    {
        let req = self
            .client
            .request(method.try_into().map_err(E::into)?, url.into_url()?);
        Ok(req)
    }

    /// Execute renders the request
    async fn execute(&self, request: reqwest::Request) -> Result<reqwest::Response> {
        self.client
            .execute(request)
            .await
            .map_err(reqwest::Error::into)
    }

    /// Name returns "default"
    fn name(&self) -> &String {
        &self.name
    }
}

impl Display for Unspecified {
    /// outputs "default"
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
