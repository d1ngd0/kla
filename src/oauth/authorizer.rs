use crate::KResult;
use oauth2::{AuthorizationCode, CsrfToken};
use url::Url;

/// Authorizer holds the logic which takes the Authorization endpoint and returns
/// the AuthorizationCode. This function is what should open a webbrowser or whatever
/// you should make sure the csrf token matches what you found in the redirectURL
pub trait Authorizer {
    fn authorize(&self, url: Url, csrf: CsrfToken) -> KResult<AuthorizationCode>;
}

/// Ability to turn a closure into an authorizer for one off stuff
impl<T: Fn(Url, CsrfToken) -> KResult<AuthorizationCode> + Send + Sync> Authorizer for T {
    fn authorize(&self, url: Url, csrf: CsrfToken) -> KResult<AuthorizationCode> {
        self(url, csrf)
    }
}
