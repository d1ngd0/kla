use crate::{impl_opt, Error, Result};

use duration_string::DurationString;
use reqwest::{header::HeaderValue, redirect::Policy, Certificate, ClientBuilder};
use std::str::FromStr;
use std::{fs, path::Path, time::Duration};

// KlaClientBuilder is a trait that adds additional functionality to the reqwest::ClientBuilder
// object. These functions make it easier to marry the functionality with Clap
pub trait KlaClientBuilder {
    fn opt_header_agent<'a>(self, agent: Option<&'a String>) -> Result<ClientBuilder>;

    fn opt_max_redirects(self, redirects: Option<&usize>) -> ClientBuilder;

    fn no_redirects(self, no_redirects: bool) -> ClientBuilder;

    fn opt_proxy(self, proxy: Option<&String>, userpass: Option<&String>) -> Result<ClientBuilder>;

    fn opt_proxy_http(
        self,
        proxy: Option<&String>,
        userpass: Option<&String>,
    ) -> Result<ClientBuilder>;

    fn opt_proxy_https(
        self,
        proxy: Option<&String>,
        userpass: Option<&String>,
    ) -> Result<ClientBuilder>;

    fn connect_timeout(self, timeout: Option<&String>) -> Result<ClientBuilder>;

    fn opt_certificate<'a, T>(self, certificates: Option<T>) -> Result<ClientBuilder>
    where
        T: Iterator<Item = &'a String>;
}

// Implementation of the trait to extend ClientBuilder
impl KlaClientBuilder for ClientBuilder {
    fn opt_certificate<'a, T>(self, certificates: Option<T>) -> Result<ClientBuilder>
    where
        T: Iterator<Item = &'a String>,
    {
        if let None = certificates {
            return Ok(self);
        }
        let certificates = certificates.unwrap();

        let mut me = self;

        for certificate in certificates {
            let ext = Path::new(certificate).extension().and_then(|s| s.to_str());
            match ext {
                Some("pem") => {
                    let pem = fs::read_to_string(certificate)?;
                    let certificate = Certificate::from_pem(pem.as_bytes())?;
                    me = me.add_root_certificate(certificate);
                }
                Some("der") => {
                    let pem = fs::read_to_string(certificate)?;
                    let certificate = Certificate::from_der(pem.as_bytes())?;
                    me = me.add_root_certificate(certificate);
                }
                _ => {
                    return Err(Error::from(format!(
                        "Invalid certificate file extension: {}",
                        certificate
                    )))
                }
            }
        }

        Ok(me)
    }

    fn no_redirects(self, no_redirects: bool) -> ClientBuilder {
        if no_redirects {
            self.redirect(Policy::none())
        } else {
            self
        }
    }

    fn opt_max_redirects(self, redirects: Option<&usize>) -> ClientBuilder {
        if let None = redirects {
            return self;
        }

        let redirects = redirects.unwrap();
        self.redirect(Policy::limited(*redirects))
    }

    fn opt_header_agent<'a>(self, agent: Option<&'a String>) -> Result<ClientBuilder> {
        if let None = agent {
            return Ok(self);
        }
        let agent = HeaderValue::from_str(agent.unwrap())?;
        Ok(self.user_agent(agent))
    }

    fn opt_proxy(self, proxy: Option<&String>, userpass: Option<&String>) -> Result<ClientBuilder> {
        if let None = proxy {
            return Ok(self);
        }

        let proxy = reqwest::Proxy::all(proxy.unwrap())?;
        if let None = userpass {
            return Ok(self.proxy(proxy));
        }

        let mut parts = userpass.unwrap().splitn(2, ":");

        Ok(self.proxy(proxy.basic_auth(parts.next().unwrap(), parts.next().unwrap_or_default())))
    }

    fn opt_proxy_http(
        self,
        proxy: Option<&String>,
        userpass: Option<&String>,
    ) -> Result<ClientBuilder> {
        if let None = proxy {
            return Ok(self);
        }

        let proxy = reqwest::Proxy::http(proxy.unwrap())?;
        if let None = userpass {
            return Ok(self.proxy(proxy));
        }

        let mut parts = userpass.unwrap().splitn(2, ":");

        Ok(self.proxy(proxy.basic_auth(parts.next().unwrap(), parts.next().unwrap_or_default())))
    }

    fn opt_proxy_https(
        self,
        proxy: Option<&String>,
        userpass: Option<&String>,
    ) -> Result<ClientBuilder> {
        if let None = proxy {
            return Ok(self);
        }

        let proxy = reqwest::Proxy::https(proxy.unwrap())?;
        if let None = userpass {
            return Ok(self.proxy(proxy));
        }

        let mut parts = userpass.unwrap().splitn(2, ":");
        Ok(self.proxy(proxy.basic_auth(parts.next().unwrap(), parts.next().unwrap_or_default())))
    }

    fn connect_timeout(self, timeout: Option<&String>) -> Result<ClientBuilder> {
        if let None = timeout {
            return Ok(self);
        }

        let timeout: Duration = match DurationString::from_str(timeout.unwrap()) {
            Ok(v) => Ok(v),
            Err(msg) => Err(Error::from(msg.as_str())),
        }?
        .into();
        Ok(self.connect_timeout(timeout))
    }
}

impl_opt!(ClientBuilder, crate::Error);
