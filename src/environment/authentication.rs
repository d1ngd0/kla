use std::sync::Arc;

use crate::{
    basic_auth::BasicAuth, bearer_token::BearerToken, config, oauth::OAuth, Result, SigV4,
};
use reqwest::{Request, RequestBuilder};
use serde::{Deserialize, Serialize};

pub trait Authentication: std::fmt::Debug + Send + Sync {
    /// authorize allows you to help craft the request prior to having one
    /// This must be implemented, even if it just returns the requestbuilder
    fn authorize(&self, builder: RequestBuilder) -> Result<RequestBuilder>;

    /// sign is an optional function that takes the incoming request
    /// and returns a new request, or the same request modified. This
    /// function is passed a fully complete request, right before we
    /// send it, so we can sign the request if the authentication
    /// method requires that.
    fn sign(&self, req: Request) -> Result<Request> {
        Ok(req)
    }
}

impl TryFrom<config::Authentication> for Arc<dyn Authentication> {
    type Error = crate::Error;

    fn try_from(value: config::Authentication) -> Result<Self> {
        match value {
            config::Authentication::SigV4(sig_v4) => {
                let val: Arc<dyn crate::Authentication> = Arc::new(SigV4::try_from(sig_v4)?);
                Ok(val)
            }
            config::Authentication::OAuth(oauth) => {
                let val: Arc<dyn crate::Authentication> = Arc::new(OAuth::try_from(oauth)?);
                Ok(val)
            }
            config::Authentication::BasicAuth(basic) => {
                let val: Arc<dyn crate::Authentication> = Arc::new(BasicAuth::try_from(basic)?);
                Ok(val)
            }
            config::Authentication::BearerToken(bearer) => {
                let val: Arc<dyn crate::Authentication> = Arc::new(BearerToken::try_from(bearer)?);
                Ok(val)
            }
            config::Authentication::None => {
                let val: Arc<dyn crate::Authentication> = Arc::new(NoopAuth {});
                Ok(val)
            }
        }
    }
}

/// Noop Authentication
#[derive(Debug, Copy, Serialize, Deserialize, Clone)]
pub struct NoopAuth {}

impl Default for NoopAuth {
    fn default() -> Self {
        Self {}
    }
}

impl Authentication for NoopAuth {
    fn authorize(&self, builder: RequestBuilder) -> Result<RequestBuilder> {
        return Ok(builder);
    }
}
