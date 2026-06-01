use std::{fs::read_to_string, sync::Arc, time::Duration};

use anyhow::Context;
use http::Version;
use reqwest::{redirect, ClientBuilder, Proxy, Request, RequestBuilder};

use crate::{config, Authentication, Error, NoopAuth, Opt, Result};

#[derive(Clone, Debug)]
pub struct Attributes {
    agent: String,
    timeout: Duration,
    connect_timeout: Duration,
    http_version: Version,
    gzip: bool,
    brotli: bool,
    deflate: bool,
    accept_invalid_certs: bool,
    accept_invalid_hostnames: bool,
    redirect_policy: Arc<redirect::Policy>,
    proxy: Option<Proxy>,
    pub auth: Arc<dyn Authentication>,
    verbose: bool,
}

const DEFAULT_AGENT: &str = "kla 0.x";
const DEFAULT_TIMEOUT_SECONDS: u64 = 900;
const DEFAULT_CONNECT_TIMEOUT_SECONDS: u64 = 5;
const DEFAULT_MAX_REDIRECTS: usize = 10;

impl Default for Attributes {
    fn default() -> Self {
        Self {
            agent: DEFAULT_AGENT.into(),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECONDS),
            connect_timeout: Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECONDS),
            http_version: Version::default(),
            gzip: false,
            brotli: false,
            deflate: false,
            accept_invalid_certs: false,
            accept_invalid_hostnames: false,
            redirect_policy: Arc::new(redirect::Policy::limited(DEFAULT_MAX_REDIRECTS)),
            proxy: None,
            auth: Arc::new(NoopAuth::default()),
            verbose: false,
        }
    }
}

impl TryFrom<&config::Attributes> for Attributes {
    type Error = crate::Error;

    fn try_from(value: &config::Attributes) -> std::result::Result<Self, Self::Error> {
        value.clone().try_into()
    }
}

impl TryFrom<config::Attributes> for Attributes {
    type Error = crate::Error;

    fn try_from(value: config::Attributes) -> std::result::Result<Self, Self::Error> {
        let proxy_auth = value
            .proxy_auth_path
            .map(|s| {
                read_to_string(s.as_path())
                    .with_context(|| format!("could not read file {:?}", s.as_path()))
            })
            .transpose()?
            .or(value.proxy_auth)
            .map(|s| {
                let mut v = s.splitn(2, ":");
                (v.next().map(String::from), v.next().map(String::from))
            });

        Ok(Attributes {
            agent: value.agent.unwrap_or(DEFAULT_AGENT.to_string()),
            timeout: value
                .timeout
                .as_ref()
                .map(duration_str::parse)
                .transpose()
                .map_err(|err| Error::from(err))?
                .unwrap_or(Duration::from_secs(DEFAULT_TIMEOUT_SECONDS)),
            connect_timeout: value
                .connect_timeout
                .as_ref()
                .map(duration_str::parse)
                .transpose()
                .map_err(|err| Error::from(err))?
                .unwrap_or(Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECONDS)),
            http_version: value
                .http_version
                .map(|f| match f.as_str() {
                    "0.9" => Ok(Version::HTTP_09),
                    "1.0" => Ok(Version::HTTP_10),
                    "1.1" => Ok(Version::HTTP_11),
                    "2.0" => Ok(Version::HTTP_2),
                    "3.0" => Ok(Version::HTTP_3),
                    _ => Err(Error::from("invalid http version")),
                })
                .transpose()?
                .unwrap_or_default(),
            // TODO these are all negated which is weird.
            gzip: !value.no_gzip,
            brotli: !value.no_brotli,
            deflate: !value.no_deflate,

            accept_invalid_certs: value.accept_invalid_certs,
            accept_invalid_hostnames: value.accept_invalid_hostnames,
            redirect_policy: Arc::new(if value.no_redirects {
                redirect::Policy::none()
            } else {
                redirect::Policy::limited(value.max_redirects.unwrap_or(DEFAULT_MAX_REDIRECTS))
            }),
            proxy: value
                .proxy_https
                .or(value.proxy)
                .or(value.proxy_http)
                .map(|p| Proxy::all(p))
                .transpose()?
                .map(|p| {
                    if let Some(proxy_auth) = proxy_auth {
                        p.basic_auth(&proxy_auth.0.unwrap(), &proxy_auth.1.unwrap_or_default())
                    } else {
                        p
                    }
                }),
            auth: value
                .auth
                .map(|auth| {
                    let auth: Result<Arc<dyn Authentication>> = auth.try_into();
                    auth
                })
                .transpose()?
                .unwrap_or_else(|| Arc::new(NoopAuth::default())),
            verbose: value.verbose,
        })
    }
}
pub trait WithAttributes: Sized {
    type Error;
    /// with_attributes allows the attributes to be applied to any builders
    fn with_attributes(self, attr: &Attributes) -> std::result::Result<Self, Self::Error>;
}

impl AsRef<Attributes> for Attributes {
    fn as_ref(&self) -> &Attributes {
        self
    }
}

impl WithAttributes for ClientBuilder {
    type Error = crate::Error;

    fn with_attributes(self, attr: &Attributes) -> std::result::Result<Self, Self::Error> {
        Ok(self
            .user_agent(attr.agent.as_str())
            .gzip(attr.gzip)
            .brotli(attr.brotli)
            .deflate(attr.deflate)
            .connection_verbose(attr.verbose)
            .connect_timeout(attr.connect_timeout)
            .with_some(attr.proxy.clone(), Self::proxy)
            .danger_accept_invalid_hostnames(attr.accept_invalid_hostnames)
            .danger_accept_invalid_certs(attr.accept_invalid_certs))
        // TODO: doesn't implement Clone so it fucking sucks
        // .redirect(attr.redirect_policy)
    }
}

impl WithAttributes for RequestBuilder {
    type Error = crate::Error;

    fn with_attributes(self, attr: &Attributes) -> std::result::Result<Self, Self::Error> {
        let b = self.timeout(attr.timeout).version(attr.http_version);
        attr.auth.authorize(b)
    }
}

impl WithAttributes for Request {
    type Error = crate::Error;

    fn with_attributes(self, attr: &Attributes) -> std::result::Result<Self, Self::Error> {
        attr.auth.sign(self)
    }
}
