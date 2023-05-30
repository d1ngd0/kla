use super::Error;
use core::fmt::Display;
use reqwest::RequestBuilder;
use std::fs;

pub enum AuthType {
    Bearer(Box<dyn Display>),
    Basic { username: String, password: String },
    None,
}

impl AuthType {
    pub fn bearer_from_string(token: String) -> AuthType {
        AuthType::Bearer(Box::new(token))
    }

    pub fn bearer_from_file(path: &str) -> Result<AuthType, Error> {
        let s = fs::read_to_string(path)?;
        Ok(AuthType::bearer_from_string(s))
    }

    pub fn basic_from_string(token: &str) -> Result<AuthType, Error> {
        let mut parts = token.splitn(2, ":");

        let username = if let Some(username) = parts.next() {
            String::from(username)
        } else {
            return Err(Error::InvalidArguments(String::from(
                "No value specified for basic authentication",
            )));
        };

        let password = if let Some(password) = parts.next() {
            String::from(password)
        } else {
            return Err(Error::InvalidArguments(String::from(
                "Invalid basic authentication, no password was provided",
            )));
        };

        Ok(AuthType::Basic { username, password })
    }

    pub fn basic_from_file(path: &str) -> Result<AuthType, Error> {
        let s = fs::read_to_string(path)?;
        AuthType::basic_from_string(&s)
    }

    pub fn apply(&self, request: RequestBuilder) -> RequestBuilder {
        match self {
            AuthType::Bearer(token) => request.bearer_auth(token),
            AuthType::Basic { username, password } => request.basic_auth(username, Some(password)),
            AuthType::None => request,
        }
    }
}
