#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Invalid JSON Body")]
    BodyParsingError(String),
    #[error("Configuration Error")]
    ConfigError(String),
    #[error("Could not create client")]
    ClientError(String),
    #[error("Could not create template")]
    TemplateError(String),
    #[error("Invalid arguments sent")]
    InvalidArguments(String),
    #[error("io Error")]
    IOError(String),
    #[error("Invalid Method")]
    InvalidMethod,
    #[error("Invalid Url")]
    InvalidURL,
    #[error("Body not UTF-8")]
    InvalidBody,
}

impl std::convert::From<reqwest::header::InvalidHeaderValue> for Error {
    fn from(err: reqwest::header::InvalidHeaderValue) -> Self {
        Error::InvalidArguments(err.to_string())
    }
}

impl std::convert::From<reqwest::header::InvalidHeaderName> for Error {
    fn from(err: reqwest::header::InvalidHeaderName) -> Self {
        Error::InvalidArguments(err.to_string())
    }
}

impl std::convert::From<regex::Error> for Error {
    fn from(err: regex::Error) -> Self {
        Error::InvalidArguments(err.to_string())
    }
}

impl std::convert::From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::ClientError(err.to_string())
    }
}

impl std::convert::From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IOError(err.to_string())
    }
}

impl std::convert::From<std::str::Utf8Error> for Error {
    fn from(_: std::str::Utf8Error) -> Self {
        Error::InvalidBody
    }
}

impl std::convert::From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::BodyParsingError(err.to_string())
    }
}

impl std::convert::From<config::ConfigError> for Error {
    fn from(err: config::ConfigError) -> Self {
        Error::ConfigError(err.to_string())
    }
}

impl std::convert::From<tera::Error> for Error {
    fn from(err: tera::Error) -> Self {
        Error::TemplateError(err.to_string())
    }
}

impl std::convert::From<http::method::InvalidMethod> for Error {
    fn from(_: http::method::InvalidMethod) -> Self {
        Error::InvalidMethod
    }
}

impl std::convert::From<url::ParseError> for Error {
    fn from(_: url::ParseError) -> Self {
        Error::InvalidURL
    }
}
