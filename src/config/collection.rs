use serde::Deserialize;

use crate::config::command::ConfigArg;

#[derive(Deserialize, Debug, Clone)]
pub struct Collection {
    group: Vec<String>,
    #[serde(default)]
    args: Vec<ConfigArg>,
    template: 
}

pub 

