use std::{collections::HashMap, ops::Deref, sync::Arc};

use oci_client::{secrets::RegistryAuth, Reference};

use crate::{config, Authentication, KResult};

#[derive(Clone, Debug, Default)]
pub struct AuthenticationBuilder {
    auth: Option<RegistryAuth>,
}

impl AuthenticationBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn basic(mut self, username: String, password: String) -> Self {
        self.auth = Some(RegistryAuth::Basic(username, password));
        self
    }

    pub fn bearer(mut self, token: String) -> Self {
        self.auth = Some(RegistryAuth::Bearer(token));
        self
    }

    pub fn build(self) -> RegistryAuth {
        self.auth.unwrap_or(RegistryAuth::Anonymous)
    }
}

pub struct AuthenticationBuilderRepo(HashMap<String, config::Authentication>);
type DynAuthenticationBuilder = Arc<dyn Authentication<AuthenticationBuilder>>;

impl AuthenticationBuilderRepo {
    // TODO: Right now we clone the config value here. Instead we should cache
    // the value in memory so if we request it again we already have it. We
    // can remove the value from the config hashmap once we have an authetication
    // built for it.
    pub fn fetch(&self, refr: &Reference) -> KResult<RegistryAuth> {
        let refr_string = refr.to_string();

        for (key, val) in self.iter() {
            if refr_string.starts_with(key) {
                let b = AuthenticationBuilder::new();
                // TODO: the clone in question is here. Val is a config::Authentication
                // which we defer to try_from until now so we don't prompt users for
                // config secrets until they actually use it
                return Ok(DynAuthenticationBuilder::try_from(val.clone())?
                    .authorize(b)?
                    .build());
            }
        }

        Ok(RegistryAuth::Anonymous)
    }
}

impl Deref for AuthenticationBuilderRepo {
    type Target = HashMap<String, config::Authentication>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Vec<config::Registry>> for AuthenticationBuilderRepo {
    type Error = crate::Error;

    fn try_from(regs: Vec<config::Registry>) -> Result<Self, Self::Error> {
        let mut registries: HashMap<String, config::Authentication> = HashMap::new();
        for reg in regs {
            registries.insert(reg.registry, reg.authentication.unwrap_or_default());
        }

        Ok(AuthenticationBuilderRepo(registries))
    }
}
