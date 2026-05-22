use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SigV4 {
    #[serde(rename = "profile")]
    pub profile: Option<String>,
    #[serde(rename = "service")]
    pub service: Option<String>,
}
