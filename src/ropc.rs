use std::{collections::HashMap, path::PathBuf};

use reqwest::{blocking::Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::{
    config::{self, CachedSecretValue},
    filecache::CacheFile,
    Authentication, Result,
};

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
    expires_in: u32,
}

#[derive(Clone, Debug)]
pub struct ROPC {
    client_id: ClientId,
    token_url: TokenUrl,
    client_secret: CachedSecretValue,
    username: CachedSecretValue,
    password: CachedSecretValue,
    token: CacheFile,
}

macro_rules! create_ropc_field {
    ($name:ident) => {
        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct $name(String);

        impl AsRef<String> for $name {
            fn as_ref(&self) -> &String {
                &self.0
            }
        }

        impl AsMut<String> for $name {
            fn as_mut(&mut self) -> &mut String {
                &mut self.0
            }
        }
    };
}

create_ropc_field!(ClientId);
create_ropc_field!(TokenUrl);

impl Authentication for ROPC {
    fn authorize(&self, builder: RequestBuilder) -> Result<RequestBuilder> {
        let token = self.token.fetch(|| self.ropc_flow())?;
        Ok(builder.bearer_auth(token))
    }
}

impl ROPC {
    fn ropc_flow(&self) -> Result<(String, chrono::Duration)> {
        let client = Client::new();
        let mut params = HashMap::new();
        let secret = self.client_secret.to_string()?;
        let username = self.username.to_string()?;
        let password = self.password.to_string()?;

        params.insert("grant_type", "password");
        params.insert("client_id", self.client_id.as_ref());
        params.insert("client_secret", &secret);
        params.insert("username", &username);
        params.insert("password", &password);

        // Make the token request
        let response = client
            .post(self.token_url.as_ref())
            .form(&params)
            .send()?
            .json::<TokenResponse>()?;

        Ok((
            response.access_token,
            chrono::Duration::seconds(response.expires_in as i64),
        ))
    }
}

impl TryFrom<config::ROPC> for ROPC {
    type Error = crate::Error;

    fn try_from(value: config::ROPC) -> std::result::Result<Self, Self::Error> {
        let mut hasher = Sha256::new();

        hasher.update(value.client_id.as_ref().as_bytes());
        hasher.update(value.token_url.as_ref().as_bytes());

        // To see it as a hex string
        let path = PathBuf::from("/tmp").join(format!("{}.token", hex::encode(hasher.finalize())));

        Ok(Self {
            client_id: value.client_id,
            token_url: value.token_url,
            client_secret: value.client_secret.cached(),
            username: value.username.cached(),
            password: value.password.cached(),
            token: CacheFile::new(path.as_path()),
        })
    }
}
