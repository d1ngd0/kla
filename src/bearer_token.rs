use crate::{
    config::{self, CachedValueSource},
    Authentication, AuthenticationBuilder,
};

#[derive(Clone, Debug)]
pub struct BearerToken {
    token: CachedValueSource,
}

impl BearerToken {
    pub fn new<T>(token: T) -> BearerToken
    where
        T: Into<CachedValueSource>,
    {
        BearerToken {
            token: token.into(),
        }
    }
}

impl TryFrom<config::BearerToken> for BearerToken {
    type Error = crate::Error;

    fn try_from(value: config::BearerToken) -> std::result::Result<Self, Self::Error> {
        Ok(BearerToken::new(value.token))
    }
}

impl Authentication<reqwest::RequestBuilder> for BearerToken {
    fn authorize(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> crate::KResult<reqwest::RequestBuilder> {
        Ok(builder.bearer_auth(self.token.to_string()?))
    }
}

impl Authentication<AuthenticationBuilder> for BearerToken {
    fn authorize(&self, builder: AuthenticationBuilder) -> crate::KResult<AuthenticationBuilder> {
        Ok(builder.bearer(self.token.to_string()?))
    }
}
