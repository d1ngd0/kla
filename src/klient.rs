use super::error::Error;
use http::HeaderMap;
use reqwest::{Body, Method, Response, Url};

impl std::convert::From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::ClientError(err.to_string())
    }
}

pub struct Client {
    client: reqwest::Client,
}

impl Client {
    pub fn new(headers: HeaderMap) -> Result<Client, Error> {
        Ok(Client {
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
