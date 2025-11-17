use std::borrow::Cow;
use std::fmt::{Display, Write};

use serde::Deserialize;
use skim::SkimItem;

use crate::config::Attributes;

#[derive(Deserialize, Debug, Clone)]
/// Endpoint is a configured environment that specifies a prefix, name, template_dir
/// etc. This struct is the configuration for the final environment that we build
pub struct Endpoint {
    #[serde(rename = "name")]
    /// The name of the environment, used when selecting etc.
    pub name: String,

    #[serde(rename = "url")]
    /// The prefix of the environment is prepended to the user supplied
    /// string
    pub prefix: Option<String>,

    #[serde(rename = "settings")]
    /// client sets the client Configurations available
    pub attr: Option<Attributes>,

    #[serde(rename = "short_description")]
    /// short_description is shown to the user when running kla envs
    pub short_description: Option<String>,

    #[serde(rename = "long_description")]
    /// long_description is shown to the user when using the fuzzy finder
    pub long_description: Option<String>,

    #[serde(rename = "template_dir")]
    /// template_dir is a string location to the directory where the templates
    /// for this environment are stored. If there is no directory this should
    /// return None
    pub template_dir: Option<String>,

    /// All the following are for AWS signing of requests. These options are
    /// applied to the request after it is built, and require usage of the
    /// WithEnvironment trait on the request itself
    #[serde(rename = "sigv4")]
    pub sigv4: Option<bool>,
    #[serde(rename = "sigv4_aws_profile")]
    pub sigv4_aws_profile: Option<String>,
    #[serde(rename = "sigv4_aws_service")]
    pub sigv4_aws_service: Option<String>,
}

impl Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:", self.name)?;

        if let Some(prefix) = self.prefix.as_ref() {
            write!(f, "[{}]", prefix)?;
        }

        write!(f, "\n")?;

        if let Some(short_description) = self.short_description.as_ref() {
            write!(f, "\t{}\n", short_description)?;
        }

        Ok(())
    }
}

impl SkimItem for Endpoint {
    fn text(&self) -> Cow<'_, str> {
        Cow::from(&self.name)
    }

    fn preview(&self, _context: skim::PreviewContext) -> skim::ItemPreview {
        let mut f = String::new();
        write!(f, "{}:", self.name).unwrap();

        if let Some(prefix) = self.prefix.as_ref() {
            write!(f, "[{}]\n", prefix).unwrap();
        }

        write!(f, "\n").unwrap();

        if let Some(long_description) = self.long_description.as_ref() {
            write!(f, "\n{}\n", long_description).unwrap();
        }
        skim::ItemPreview::Text(f)
    }
}
