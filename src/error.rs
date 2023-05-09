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
    #[error("Invalid Method")]
    InvalidMethod,
    #[error("Invalid Url")]
    InvalidURL,
    #[error("Body not UTF-8")]
    InvalidBody,
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
