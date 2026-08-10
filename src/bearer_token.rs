use crate::{
    config::{self, CachedSecretValue},
    Authentication,
};

#[derive(Clone, Debug)]
pub struct BearerToken {
    token: CachedSecretValue,
}

impl BearerToken {
    pub fn new<T>(token: T) -> BearerToken
    where
        T: Into<CachedSecretValue>,
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

impl Authentication for BearerToken {
    fn authorize(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> crate::KResult<reqwest::RequestBuilder> {
        Ok(builder.bearer_auth(self.token.to_string()?))
    }
}
