use std::future::Future;
use std::{ffi::OsString, path::PathBuf};

use std::fs::{self, DirEntry};

use http::Method;
use reqwest::{ClientBuilder, IntoUrl, Request, Response};
use tera::Context;

use crate::{Error, Result};

mod optional;
pub use optional::*;
mod specified;
pub use specified::*;
mod unspecified;
pub use unspecified::*;
mod caching_loader;
pub use caching_loader::*;

/// Environment trait
pub trait Environment: Send + Sync + std::fmt::Debug {
    /// request should return a RequestBuilder with any environment specific configurations
    /// already applied. It is expected that the implementation of environment already have
    /// a client created with any environment level specifics applied as well.
    fn request<E, M, U>(&self, method: M, url: U) -> Result<reqwest::RequestBuilder>
    where
        E: Into<crate::Error>,
        M: TryInto<Method, Error = E>,
        U: IntoUrl;

    fn execute(&self, request: Request) -> impl Future<Output = Result<Response>>;

    // name returns the name of the client
    fn name(&self) -> &String;

    /// template_dir should return the location of the template directory, the default
    /// implementation returns None
    fn template_dir(&self) -> Option<&String> {
        None
    }

    /// templates iterates over the template_dir and returns each path it finds
    fn templates(&self) -> Result<Box<dyn Iterator<Item = String>>> {
        let template_dir = match self.template_dir() {
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

    // tmpl_path is given the name of a template and renders the path for it.
    // The function just appends the name to the template directory.
    fn tmpl_path(&self, name: &str) -> Result<PathBuf> {
        let name = if name.ends_with(".toml") {
            name.into()
        } else {
            let mut name = String::from(name);
            name.push_str(".toml");
            name
        };

        // create the path
        let mut path = PathBuf::from(self.template_dir().ok_or_else(|| {
            Error::from(format!(
                "{} does not have a configured template directory",
                self.name()
            ))
        })?);
        path.push(name);

        Ok(path)
    }

    /// some environments may be configured to sign the request after it has
    /// been created. This makes sure that can happen. If you think you
    /// will sign or not call this function, since it just returns the provided
    /// request with the default implementation.
    fn sign(&self, req: Request) -> Result<Request> {
        Ok(req)
    }

    /// context will take a context and add additional context to it from the
    /// environment.
    fn context(&self, context: Context) -> Result<Context> {
        Ok(context)
    }
}

/// EnvironmentLoader Enables a type to load an environment from a name
/// Since there are multiple environment types you must specify
/// the type of environment you expect this loader to generate
pub trait EnvironmentLoader<E: Environment> {
    #[allow(async_fn_in_trait)]
    async fn async_load_environment_with_priority<S, F>(self, env: S, overrides: F) -> Result<E>
    where
        S: AsRef<str>,
        F: FnOnce(ClientBuilder) -> Result<ClientBuilder>;

    fn load_environment_with_priority<S, F>(self, env: S, overrides: F) -> Result<E>
    where
        S: AsRef<str>,
        F: FnOnce(ClientBuilder) -> Result<ClientBuilder>,
        Self: Sized,
    {
        futures::executor::block_on(self.async_load_environment_with_priority(env, overrides))
    }

    #[allow(async_fn_in_trait)]
    /// load_environment allows the called to get an environment without providing overrides
    /// to the client builder
    async fn async_load_environment<S>(self, env: S) -> Result<E>
    where
        S: AsRef<str>,
        Self: Sized,
    {
        self.async_load_environment_with_priority(env, |cb| Ok(cb))
            .await
    }

    fn load_environment<S>(self, env: S) -> Result<E>
    where
        S: AsRef<str>,
        Self: Sized,
    {
        futures::executor::block_on(self.async_load_environment(env))
    }
}
