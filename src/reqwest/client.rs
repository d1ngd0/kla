use crate::{impl_opt, Error, KResult};

use reqwest::{Certificate, ClientBuilder};
use std::path::PathBuf;
use std::{fs, path::Path, time::Duration};

// KlaClientBuilder is a trait that adds additional functionality to the reqwest::ClientBuilder
// object. These functions make it easier to marry the functionality with Clap
pub trait KlaClientBuilder {
    fn opt_proxy(self, proxy: Option<&String>, userpass: Option<&String>)
        -> KResult<ClientBuilder>;

    fn opt_proxy_http(
        self,
        proxy: Option<&String>,
        userpass: Option<&String>,
    ) -> KResult<ClientBuilder>;

    fn opt_proxy_https(
        self,
        proxy: Option<&String>,
        userpass: Option<&String>,
    ) -> KResult<ClientBuilder>;

    fn opt_connect_timeout(self, timeout: Option<&String>) -> KResult<ClientBuilder>;

    fn opt_certificate<'a, T>(self, certificates: Option<T>) -> KResult<ClientBuilder>
    where
        T: Iterator<Item = &'a PathBuf>;
}

// Implementation of the trait to extend ClientBuilder
impl KlaClientBuilder for ClientBuilder {
    fn opt_certificate<'a, T>(self, certificates: Option<T>) -> KResult<ClientBuilder>
    where
        T: Iterator<Item = &'a PathBuf>,
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
                        certificate.to_string_lossy()
                    )))
                }
            }
        }

        Ok(me)
    }

    fn opt_proxy(
        self,
        proxy: Option<&String>,
        userpass: Option<&String>,
    ) -> KResult<ClientBuilder> {
        let proxy = if let Some(proxy) = proxy {
            reqwest::Proxy::all(proxy)?
        } else {
            return Ok(self);
        };

        match userpass {
            Some(userpass) => {
                let mut parts = userpass.splitn(2, ":");

                Ok(self.proxy(
                    proxy.basic_auth(parts.next().unwrap(), parts.next().unwrap_or_default()),
                ))
            }
            None => Ok(self.proxy(proxy)),
        }
    }

    fn opt_proxy_http(
        self,
        proxy: Option<&String>,
        userpass: Option<&String>,
    ) -> KResult<ClientBuilder> {
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
    ) -> KResult<ClientBuilder> {
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

    fn opt_connect_timeout(self, timeout: Option<&String>) -> KResult<ClientBuilder> {
        if let None = timeout {
            return Ok(self);
        }

        let timeout: Duration = match duration_str::parse(timeout.unwrap()) {
            Ok(v) => Ok(v),
            Err(msg) => Err(Error::from(msg.as_str())),
        }?
        .into();
        Ok(self.connect_timeout(timeout))
    }
}

impl_opt!(ClientBuilder, crate::Error);
// impl_when!(ClientBuilder);
