use http::HeaderMap;
use reqwest::{Body, Method, Response, Url};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Could not create client")]
    ClientError(String),
}

impl std::convert::From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::ClientError(err.to_string())
    }
}

pub struct Client {
    prefix: String,
    client: reqwest::Client,
}

impl Client {
    pub fn new(prefix: &str, headers: HeaderMap) -> Result<Client, Error> {
        Ok(Client {
            prefix: String::from(prefix),
            client: reqwest::Client::builder()
                .default_headers(headers)
                .build()?,
        })
    }

    pub async fn send(
        &self,
        method: Method,
        url: Url,
        body: Option<Body>,
    ) -> Result<Response, Error> {
        let mut builder = self.client.request(method, url);

        if let Some(body) = body {
            builder = builder.body(body);
        }

        Ok(builder.send().await?)
    }
}
