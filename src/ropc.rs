use std::{collections::HashMap, path::PathBuf};

use reqwest::{blocking::Client, RequestBuilder};
use serde::Deserialize;
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
    client_id: CachedSecretValue,
    token_url: String,
    client_secret: CachedSecretValue,
    username: CachedSecretValue,
    password: CachedSecretValue,
    token: CacheFile,
}

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
        let client_id = self.client_id.to_string()?;
        let secret = self.client_secret.to_string()?;
        let username = self.username.to_string()?;
        let password = self.password.to_string()?;

        params.insert("grant_type", "password");
        params.insert("client_id", &client_id);
        params.insert("client_secret", &secret);
        params.insert("username", &username);
        params.insert("password", &password);

        // Make the token request
        let response = client
            .post(self.token_url.as_str())
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

        hasher.update(format!("{:?}", value.client_id.as_ref()).as_bytes());
        hasher.update(value.token_url.as_bytes());

        // To see it as a hex string
        let path = PathBuf::from("/tmp").join(format!("{}.token", hex::encode(hasher.finalize())));

        Ok(Self {
            client_id: value.client_id.cached(),
            token_url: value.token_url,
            client_secret: value.client_secret.cached(),
            username: value.username.cached(),
            password: value.password.cached(),
            token: CacheFile::new(path.as_path()),
        })
    }
}
