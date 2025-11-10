use std::{
    borrow::Cow,
    ffi::OsString,
    fmt::{Display, Write},
    path::PathBuf,
};

use std::fs::{self, DirEntry};

use config::{builder::DefaultState, Config, ConfigBuilder, File};
use reqwest::{Client, ClientBuilder, Request, RequestBuilder};
use serde::Deserialize;
use skim::SkimItem;

use crate::{
    url_builder::{AssumingURLBuilder, OptBaseURLBuilder},
    Error, Expand, Result, Sigv4Request,
};

mod specified;

/// Environment trait
pub trait Environment: Display {
    /// request should return a RequestBuilder with any environment specific configurations
    /// already applied. It is expected that the implementation of environment already have
    /// a client created with any environment level specifics applied as well.
    fn request(&self) -> RequestBuilder;

    /// Name should return a name for the client, the default implementation just returns
    /// "default"
    fn name(&self) -> String {
        "default".into()
    }

    /// template_dir should return the location of the template directory, the default
    /// implementation returns None
    fn template_dir(&self) -> Option<&String> {
        None
    }

    /// templates returns an Iterator over each template, the default
    /// implementation returns an empty iterator
    fn templates(&self) -> Result<Box<dyn Iterator<Item = String>>> {
        Ok(Box::new(std::iter::empty()))
    }

    /// some environments may be configured to sign the request after it has
    /// been created. This makes sure that can happen. If you think you
    /// will sign or not call this function, since it just returns the provided
    /// request with the default implementation.
    fn sign(&self, req: Request) -> Result<Request> {
        Ok(req)
    }
}

// #[derive(Debug)]
// pub enum Environment {
//     Endpoint(Endpoint),
//     Empty,
// }

// impl Default for Environment {
//     fn default() -> Self {
//         return Environment::Empty;
//     }
// }

// impl Display for Environment {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Self::Endpoint(endpoint) => endpoint.fmt(f),
//             Self::Empty => Ok(()),
//         }
//     }
// }

// impl Environment {
//     pub fn new(env: Option<&String>, config: &Config) -> Result<Environment> {
//         if let Some(env) = env {
//             Ok(Environment::Endpoint(Endpoint::new(env.clone(), config)?))
//         } else {
//             Ok(Environment::Empty)
//         }
//     }

//     pub fn url_builder(&self) -> OptBaseURLBuilder {
//         match self {
//             Environment::Endpoint(endpoint) => OptBaseURLBuilder::from(endpoint.url_builder()),
//             Environment::Empty => OptBaseURLBuilder::empty(),
//         }
//     }

//     pub fn template_dir(&self) -> Option<&String> {
//         match self {
//             Environment::Endpoint(endpoint) => endpoint.template_dir(),
//             Environment::Empty => None,
//         }
//     }

//     pub fn templates(&self) -> Result<Box<dyn Iterator<Item = String>>> {
//         match self {
//             Environment::Endpoint(endpoint) => endpoint.walk_templates(),
//             Environment::Empty => Ok(Box::new(std::iter::empty())),
//         }
//     }

//     pub fn name(&self) -> Option<&String> {
//         match self {
//             Environment::Endpoint(endpoint) => Some(&endpoint.name),
//             Environment::Empty => None,
//         }
//     }
// }

impl Endpoint {
    /// new creates a new environment with the clientBuilder specified. The environment
    /// will add any additional settings specified in the environment to the client
    /// before building and storing it.
    pub fn new<S>(env: S, config: &Config, client: ClientBuilder) -> Result<Endpoint>
    where
        S: Into<String>,
    {
        let env: String = env.into();
        let mut endpoint =
            config.get::<Endpoint>(format!("environment.{}", env.as_str()).as_ref())?;

        // set the name
        endpoint.name = env;

        // normalize the prefix
        if !endpoint.prefix.ends_with("/") {
            endpoint.prefix.push_str("/");
        };

        endpoint.template_dir = endpoint.template_dir.map(String::shell_expansion);

        Ok(endpoint)
    }

    pub fn url_builder(&self) -> AssumingURLBuilder {
        AssumingURLBuilder::from(&self.prefix)
    }

    // template_dir returns the directory for the given environment
    pub fn template_dir(&self) -> Option<&String> {
        self.template_dir.as_ref()
    }

    /// walk_templates returns a WalkDir of all the templates in the
    /// template directory
    pub fn walk_templates(&self) -> Result<Box<dyn Iterator<Item = String>>> {
        let template_dir = match self.template_dir.as_ref() {
            Some(template) => template,
            None => return Ok(Box::new(std::iter::empty())),
        };

        let templates = fs::read_dir(template_dir)?
            .collect::<std::result::Result<Vec<DirEntry>, std::io::Error>>()?
            .into_iter()
            .filter(|f| f.file_type().map(|v| v.is_file()).unwrap_or(false))
            .filter_map(|f| OsString::from(f.path().file_stem()?).into_string().ok());

        Ok(Box::new(templates))
    }
}

impl Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: [{}]\n", self.name, self.prefix)?;

        if let Some(short_description) = self.short_description.as_ref() {
            write!(f, "\tdescription: {}\n", short_description)?;
        }

        Ok(())
    }
}

impl SkimItem for Endpoint {
    fn text(&self) -> Cow<'_, str> {
        Cow::from(&self.name)
    }

    fn preview(&self, _context: skim::PreviewContext) -> skim::ItemPreview {
        let mut s = String::new();
        write!(s, "{}: [{}]\n", &self.name, &self.prefix).expect("writing to string");

        if let Some(long_description) = self.long_description.as_ref() {
            write!(s, "\n{long_description}").expect("writing to string");
        } else if let Some(short_description) = self.short_description.as_ref() {
            write!(s, "\n{short_description}").expect("writing to string");
        }

        skim::ItemPreview::Text(s)
    }
}

pub trait FromEnvironment {
    type Output;

    fn add_source_environment(self, env: &crate::Environment, tmpl: &str) -> Result<Self::Output>;
}

impl FromEnvironment for ConfigBuilder<DefaultState> {
    type Output = Self;

    fn add_source_environment(self, env: &Environment, tmpl: &str) -> Result<Self::Output> {
        let environment = match env {
            Environment::Empty => return Err(Error::from("no environment set")),
            Environment::Endpoint(endpoint) => endpoint,
        };

        let template_dir = match environment.template_dir.as_ref() {
            Some(val) => val,
            None => return Err(Error::from("no template directory set")),
        };

        let mut template = PathBuf::from(template_dir);
        template.push(tmpl);

        Ok(self.add_source(File::with_name(
            template.as_path().to_str().expect("bad path"),
        )))
    }
}

pub trait WithEnvironment: Sized {
    fn with_environment(
        self,
        env: &Environment,
    ) -> impl std::future::Future<Output = Result<Self>> + Send;
}

impl WithEnvironment for ClientBuilder {
    async fn with_environment(self, _env: &Environment) -> Result<Self> {
        Ok(self)
    }
}

impl WithEnvironment for RequestBuilder {
    async fn with_environment(self, _env: &Environment) -> Result<Self> {
        Ok(self)
    }
}

impl WithEnvironment for Request {
    async fn with_environment(self, env: &Environment) -> Result<Self> {
        let endpoint = match env {
            Environment::Endpoint(endpoint) => endpoint,
            Environment::Empty => return Ok(self),
        };

        let request = if endpoint.sigv4.unwrap_or(false) {
            self.sign_request(
                endpoint.sigv4_aws_profile.as_ref(),
                endpoint.sigv4_aws_service.as_ref(),
            )
            .await?
        } else {
            self
        };

        Ok(request)
    }
}
