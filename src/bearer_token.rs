use crate::{config, Authentication};

#[derive(Clone, Debug)]
pub struct BearerToken {
    token: String,
}

impl BearerToken {
    pub fn new<T>(token: T) -> BearerToken
    where
        T: Into<String>,
    {
        BearerToken {
            token: token.into(),
        }
    }
}

impl TryFrom<config::BearerToken> for BearerToken {
    type Error = crate::Error;

    fn try_from(value: config::BearerToken) -> std::result::Result<Self, Self::Error> {
        Ok(BearerToken::new(String::try_from(value.token)?))
    }
}

impl Authentication for BearerToken {
    fn authorize(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> crate::Result<reqwest::RequestBuilder> {
        Ok(builder.bearer_auth(self.token.as_str()))
    }
}
