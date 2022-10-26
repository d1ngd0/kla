use http::HeaderMap;
use reqwest::{ Url, Method, Body };

#[derive(thiserror::Error)]
#[derive(Debug)]
pub enum Error {
    #[error("Could not create client")]
    ClientError(String)
}

impl std::convert::From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::ClientError(err.to_string())
    }
}

pub struct Client {
    prefix: String,
    client:  reqwest::Client,
}

impl Client {
    fn new(prefix: &str, headers: HeaderMap) -> Result<Client, Error> {
        Ok(Client{ 
            prefix: String::from(prefix),
            client: reqwest::Client::builder()
                .default_headers(headers)
                .build()?
        })
    }

    fn Send(&self, method: Method, url: Url, body: Option<Body>) -> 
}
