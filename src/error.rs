use std::convert::{From, Infallible};
use std::error::Error as StdError;
use std::sync::PoisonError;

use oauth2::basic::BasicErrorResponseType;
use oauth2::{HttpClientError, RequestTokenError, StandardErrorResponse};

use crate::sigv4::SigningError;
use crate::TokenFileContents;

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) type BoxError = Box<dyn StdError + Send + Sync>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Error Parsing Data: {0}")]
    BodyParsingError(#[from] serde_json::Error),
    #[error("HTTP Error: {0}")]
    HTTPError(#[from] reqwest::Error),
    #[error("Templating Error: {0}")]
    TemplateError(#[from] tera::Error),
    #[error("Invalid arguments: {0}")]
    InvalidArguments(BoxError),
    #[error("io Error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Body not UTF-8: {0}")]
    InvalidBody(#[from] std::str::Utf8Error),
    #[error("skim error: {0}")]
    SkimError(#[from] skim::options::SkimOptionsBuilderError),
    #[error("{0}")]
    KlaError(String),
    #[error("{0}")]
    Error(#[from] anyhow::Error),
    #[error("{0}")]
    SigningError(#[from] SigningError),
    #[error("Toml Parse Error {0}")]
    TomlError(#[from] toml::de::Error),
    #[error("aint never gonna happen")]
    Infallable(#[from] Infallible),
    #[error("{0}")]
    LockingError(#[from] PoisonError<TokenFileContents>),
    #[error("{0}")]
    OAuthError(
        #[from]
        RequestTokenError<
            HttpClientError<reqwest::Error>,
            StandardErrorResponse<BasicErrorResponseType>,
        >,
    ),
}

impl From<&str> for Error {
    fn from(err: &str) -> Self {
        Error::KlaError(err.to_string())
    }
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Error::KlaError(err)
    }
}

impl From<regex::Error> for Error {
    fn from(err: regex::Error) -> Self {
        Error::InvalidArguments(Box::new(err))
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Error::InvalidArguments(Box::new(err))
    }
}

impl From<http::method::InvalidMethod> for Error {
    fn from(err: http::method::InvalidMethod) -> Self {
        Error::InvalidArguments(Box::new(err))
    }
}

impl From<reqwest::header::ToStrError> for Error {
    fn from(err: reqwest::header::ToStrError) -> Self {
        Error::InvalidArguments(Box::new(err))
    }
}

impl From<reqwest::header::InvalidHeaderValue> for Error {
    fn from(err: reqwest::header::InvalidHeaderValue) -> Self {
        Error::InvalidArguments(Box::new(err))
    }
}

impl From<reqwest::header::InvalidHeaderName> for Error {
    fn from(err: reqwest::header::InvalidHeaderName) -> Self {
        Error::InvalidArguments(Box::new(err))
    }
}
