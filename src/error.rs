use std::convert::{From, Infallible};
use std::fmt::Display;

use inquire::InquireError;
use oauth2::basic::BasicErrorResponseType;
use oauth2::{HttpClientError, RequestTokenError, StandardErrorResponse};

use crate::sigv4::SigningError;

pub type KResult<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Error Parsing Data: {0}")]
    BodyParsingError(#[from] serde_json::Error),
    #[error("HTTP Error: {0}")]
    HTTPError(#[from] reqwest::Error),
    #[error("Templating Error: {0}")]
    TemplateError(#[from] tera::Error),
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
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
    TomlDeserializeError(#[from] toml::de::Error),
    #[error("{0}")]
    TOMLSerializeError(#[from] toml::ser::Error),
    #[error("aint never gonna happen")]
    Infallable(#[from] Infallible),
    #[error("{0}")]
    OAuthError(
        #[from]
        RequestTokenError<
            HttpClientError<reqwest::Error>,
            StandardErrorResponse<BasicErrorResponseType>,
        >,
    ),
    #[error("{0}")]
    KeyError(#[from] rcgen::Error),
    #[error("{0}")]
    GenError(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error("{0}")]
    PromptError(#[from] InquireError),
    #[error("{0}")]
    OCIError(#[from] oci_client::errors::OciDistributionError),
    #[error("{0}")]
    SemVerError(#[from] semver::Error),
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
        Error::invalid_arguments(err)
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Error::invalid_arguments(err)
    }
}

impl From<http::method::InvalidMethod> for Error {
    fn from(err: http::method::InvalidMethod) -> Self {
        Error::invalid_arguments(err)
    }
}

impl From<reqwest::header::ToStrError> for Error {
    fn from(err: reqwest::header::ToStrError) -> Self {
        Error::invalid_arguments(err)
    }
}

impl From<reqwest::header::InvalidHeaderValue> for Error {
    fn from(err: reqwest::header::InvalidHeaderValue) -> Self {
        Error::invalid_arguments(err)
    }
}

impl From<reqwest::header::InvalidHeaderName> for Error {
    fn from(err: reqwest::header::InvalidHeaderName) -> Self {
        Error::invalid_arguments(err)
    }
}

impl Error {
    pub fn invalid_arguments<D: Display>(d: D) -> Error {
        Error::InvalidArguments(format!("{}", d))
    }
}

impl Error {
    /// from_display takes any display type and turns it into an error
    pub fn from_display<D: Display>(d: D) -> Error {
        Error::KlaError(format!("{}", d))
    }
}

/// Context provides patterns seen in anyhow to wrap errors with additional
/// context without having to pull in all the anyhow code
pub trait Context<T> {
    /// Wrap the error value with additional context.
    fn context<C>(self, context: C) -> KResult<T>
    where
        C: Display + Send + Sync + 'static;

    /// Wrap the error value with additional context that is evaluated lazily
    /// only once an error does occur.
    fn with_context<C, F>(self, f: F) -> KResult<T>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C;
}

/// Here we implement the Context function for any Option types
impl<T> Context<T> for Option<T> {
    fn context<C>(self, context: C) -> KResult<T>
    where
        C: Display + Send + Sync + 'static,
    {
        match self {
            Some(v) => Ok(v),
            None => Err(Error::from_display(context)),
        }
    }

    fn with_context<C, F>(self, f: F) -> KResult<T>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        match self {
            Some(v) => Ok(v),
            None => Err(Error::from_display(f())),
        }
    }
}

// And now we create the result for any errors that can be converted into
// our packages Error. No free lunch here!
impl<T, E: Into<Error>> Context<T> for Result<T, E> {
    fn context<C>(self, context: C) -> KResult<T>
    where
        C: Display + Send + Sync + 'static,
    {
        match self {
            Ok(val) => Ok(val),
            Err(err) => Err(Error::from(format!("{}: {}", context, err.into()))),
        }
    }

    fn with_context<C, F>(self, f: F) -> KResult<T>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        match self {
            Ok(val) => Ok(val),
            Err(err) => Err(Error::from(format!("{}: {}", f(), err.into()))),
        }
    }
}
