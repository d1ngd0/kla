use crate::{
    config::{self, CachedValueSource},
    Authentication, AuthenticationBuilder,
};

#[derive(Clone, Debug)]
pub struct BasicAuth {
    username: CachedValueSource,
    password: Option<CachedValueSource>,
}

impl BasicAuth {
    pub fn new<U, P>(username: U, password: Option<P>) -> BasicAuth
    where
        U: Into<CachedValueSource>,
        P: Into<CachedValueSource>,
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

impl Authentication<AuthenticationBuilder> for BasicAuth {
    fn authorize(&self, builder: AuthenticationBuilder) -> crate::KResult<AuthenticationBuilder> {
        Ok(builder.basic(
            self.username.to_string()?,
            self.password
                .as_ref()
                .map(CachedValueSource::to_string)
                .transpose()?
                .unwrap_or_default(),
        ))
    }
}

impl Authentication<reqwest::RequestBuilder> for BasicAuth {
    fn authorize(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> crate::KResult<reqwest::RequestBuilder> {
        Ok(builder.basic_auth(
            self.username.to_string()?,
            self.password
                .as_ref()
                .map(CachedValueSource::to_string)
                .transpose()?,
        ))
    }
}
