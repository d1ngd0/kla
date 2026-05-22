use std::borrow::Cow;
use std::fmt::{Display, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use skim::SkimItem;
use tera::Value;

use crate::config::{Attributes, OAuth};

use super::SigV4;

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

    #[serde(rename = "settings", default)]
    /// client sets the client Configurations available
    pub attr: Attributes,

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
    pub template_dir: Option<PathBuf>,

    #[serde(rename = "context", default)]
    pub context: Value,
}

impl Endpoint {
    /// resolve_working_dir will find any relative links and turn them
    /// into absolute links with the provided base
    pub fn resolve_working_dir<P: AsRef<Path>>(&mut self, dir: P) {
        self.attr.resolve_working_dir(dir.as_ref());

        self.template_dir = self.template_dir.take().map(|f| {
            if f.is_relative() {
                PathBuf::from(dir.as_ref()).join(f)
            } else {
                f
            }
        });
    }
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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum Authentication {
    SigV4(SigV4),
    OAuth(OAuth),
    None,
}

impl Default for Authentication {
    fn default() -> Self {
        return Authentication::None;
    }
}

impl Authentication {
    pub fn resolve_working_dir<P: AsRef<Path>>(&mut self, dir: P) {
        match self {
            Authentication::SigV4(_) => (),
            Authentication::OAuth(oauth) => oauth.resolve_working_dir(dir.as_ref()),
            Authentication::None => (),
        }
    }
}
