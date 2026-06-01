use anyhow::Context;

use crate::{
    config::{self, CachedSecretValue},
    Authentication, Error, Result,
};

#[derive(Clone, Debug)]
pub struct BasicAuth {
    username: CachedSecretValue,
    password: Option<CachedSecretValue>,
}

impl BasicAuth {
    pub fn from_userpass_string(userpass: &str) -> Result<BasicAuth> {
        let mut chunks = userpass.splitn(2, ":");

        Ok(BasicAuth {
            username: chunks
                .next()
                .context("username not provided in string")?
                .into(),
            password: chunks.next().map(CachedSecretValue::from),
        })
    }

    pub fn new<U, P>(username: U, password: Option<P>) -> BasicAuth
    where
        U: Into<CachedSecretValue>,
        P: Into<CachedSecretValue>,
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
        Ok(BasicAuth::new(
            value.username,
            value.password.map(String::try_from).transpose()?,
        ))
    }
}

impl Authentication for BasicAuth {
    fn authorize(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> crate::Result<reqwest::RequestBuilder> {
        Ok(builder.basic_auth(
            self.username.to_string()?,
            self.password
                .as_ref()
                .map(CachedSecretValue::to_string)
                .transpose()?,
        ))
    }
}
