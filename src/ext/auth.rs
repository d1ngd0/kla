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

pub struct AuthenticationBuilderRepo(
    HashMap<String, Arc<dyn Authentication<AuthenticationBuilder>>>,
);

impl AuthenticationBuilderRepo {
    pub fn fetch(&self, refr: &Reference) -> KResult<RegistryAuth> {
        let refr_string = refr.to_string();

        for (key, val) in self.iter() {
            if refr_string.starts_with(key) {
                let b = AuthenticationBuilder::new();
                return Ok(val.authorize(b)?.build());
            }
        }

        Ok(RegistryAuth::Anonymous)
    }
}

impl Deref for AuthenticationBuilderRepo {
    type Target = HashMap<String, Arc<dyn Authentication<AuthenticationBuilder>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Vec<config::Registry>> for AuthenticationBuilderRepo {
    type Error = crate::Error;

    fn try_from(regs: Vec<config::Registry>) -> Result<Self, Self::Error> {
        let mut registries: HashMap<String, Arc<dyn Authentication<AuthenticationBuilder>>> =
            HashMap::new();
        for reg in regs {
            registries.insert(
                reg.registry,
                reg.authentication.unwrap_or_default().try_into()?,
            );
        }

        Ok(AuthenticationBuilderRepo(registries))
    }
}
