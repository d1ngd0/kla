use anyhow::Context;

use crate::{config, Authentication, Error, Result};

#[derive(Clone, Debug)]
pub struct BasicAuth {
    username: String,
    password: Option<String>,
}

impl BasicAuth {
    pub fn from_userpass_string(userpass: &str) -> Result<BasicAuth> {
        let mut chunks = userpass.splitn(2, ":");

        Ok(BasicAuth {
            username: chunks
                .next()
                .context("username not provided in string")?
                .to_string(),
            password: chunks.next().map(String::from),
        })
    }

    pub fn new<U, P>(username: U, password: Option<P>) -> BasicAuth
    where
        U: Into<String>,
        P: Into<String>,
    {
        BasicAuth {
            username: username.into(),
            password: password.map(P::into),
        }
    }
}

impl TryFrom<config::BasicAuth> for BasicAuth {
    type Error = crate::Error;

    fn try_from(value: config::BasicAuth) -> std::result::Result<Self, Self::Error> {
        if let Some(userpass) = value.userpass {
            let userpass: String = userpass.try_into()?;
            Self::from_userpass_string(&userpass)
        } else {
            Ok(BasicAuth::new(
                value.username.ok_or_else(|| {
                    Error::from("username must be supplied when using basic auth")
                })?,
                value.password.map(String::try_from).transpose()?,
            ))
        }
    }
}

impl Authentication for BasicAuth {
    fn authorize(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> crate::Result<reqwest::RequestBuilder> {
        Ok(builder.basic_auth(self.username.as_str(), self.password.as_ref()))
    }
}
