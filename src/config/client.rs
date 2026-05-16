use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
};

use crate::{KlaClientBuilder, KlaRequestBuilder, Opt, Result, When};
use anyhow::Context;
use reqwest::{redirect::Policy, ClientBuilder, RequestBuilder};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Attributes specify settings which can be configured
pub struct Attributes {
    agent: Option<String>,
    timeout: Option<String>,
    basic_auth: Option<String>,
    basic_auth_path: Option<PathBuf>,
    bearer_token: Option<String>,
    bearer_token_path: Option<PathBuf>,
    http_version: Option<String>,
    no_gzip: Option<bool>,
    no_brotli: Option<bool>,
    no_deflate: Option<bool>,
    max_redirects: Option<usize>,
    no_redirects: Option<bool>,
    proxy: Option<String>,
    proxy_http: Option<String>,
    proxy_https: Option<String>,
    proxy_auth: Option<String>,
    proxy_auth_path: Option<PathBuf>,
    connect_timeout: Option<String>,
    // TODO: fix this so it isn't public
    pub sigv4: Option<bool>,
    pub sigv4_aws_profile: Option<String>,
    pub sigv4_aws_service: Option<String>,
    accept_invalid_certs: Option<bool>,
    accept_invalid_hostnames: Option<bool>,
    #[serde(default)]
    certificate: Vec<PathBuf>,
    verbose: Option<bool>,
}

impl Attributes {
    pub fn resolve_working_dir<P: AsRef<Path>>(&mut self, dir: P) {
        self.basic_auth_path = self.basic_auth_path.take().map(|f| {
            if f.is_relative() {
                PathBuf::from(dir.as_ref()).join(f)
            } else {
                f
            }
        });

        self.bearer_token_path = self.bearer_token_path.take().map(|f| {
            if f.is_relative() {
                PathBuf::from(dir.as_ref()).join(f)
            } else {
                f
            }
        });

        self.proxy_auth_path = self.proxy_auth_path.take().map(|f| {
            if f.is_relative() {
                PathBuf::from(dir.as_ref()).join(f)
            } else {
                f
            }
        });

        for cert in &mut self.certificate {
            if cert.is_relative() {
                *cert = PathBuf::from(dir.as_ref()).join(cert.as_path())
            }
        }
    }
}

pub trait WithAttributes: Sized {
    /// with_attributes allows the attributes to be applied to any builders
    fn with_attributes(self, attr: &Attributes) -> Result<Self>;
}

impl WithAttributes for ClientBuilder {
    fn with_attributes(self, attr: &Attributes) -> Result<Self> {
        let builder = self
            .with_some(attr.agent.as_ref(), ClientBuilder::user_agent)
            .with_some(attr.no_gzip.map(|b| !b), ClientBuilder::gzip)
            .with_some(attr.no_brotli.map(|b| !b), ClientBuilder::brotli)
            .with_some(attr.no_deflate.map(|b| !b), ClientBuilder::deflate)
            .with_some(attr.no_redirects, |b, v| {
                b.when(v, |b| b.redirect(Policy::none()))
            })
            .with_some(attr.max_redirects, |b, redirects| {
                b.redirect(Policy::limited(redirects))
            })
            .with_some(attr.verbose, ClientBuilder::connection_verbose)
            .opt_connect_timeout(attr.connect_timeout.as_ref())?
            .opt_proxy(
                attr.proxy.as_ref(),
                attr.proxy_auth.as_ref().or(attr
                    .proxy_auth_path
                    .as_ref()
                    .map(read_to_string)
                    .transpose()?
                    .as_ref()),
            )
            .with_context(|| {
                format!(
                    "could not add proxy: --proxy={:?} --proxy-auth={:?}",
                    attr.proxy.as_ref(),
                    attr.proxy_auth
                        .as_ref()
                        .map(|v| "*".repeat(v.len()))
                        .or(attr
                            .proxy_auth_path
                            .as_ref()
                            .map(|s| s.to_string_lossy().into()))
                )
            })?
            .opt_proxy_http(attr.proxy_http.as_ref(), attr.proxy_auth.as_ref())
            .with_context(|| {
                format!(
                    "could not add proxy: --proxy-http={:?} --proxy-auth={:?}",
                    attr.proxy_http.as_ref(),
                    attr.proxy_auth
                        .as_ref()
                        .map(|v| "*".repeat(v.len()))
                        .or(attr
                            .proxy_auth_path
                            .as_ref()
                            .map(|s| s.to_string_lossy().into()))
                )
            })?
            .opt_proxy_https(attr.proxy_https.as_ref(), attr.proxy_auth.as_ref())
            .with_context(|| {
                format!(
                    "could not add proxy: --proxy-https={:?} --proxy-auth={:?}",
                    attr.proxy_https.as_ref(),
                    attr.proxy_auth
                        .as_ref()
                        .map(|v| "*".repeat(v.len()))
                        .or(attr
                            .proxy_auth_path
                            .as_ref()
                            .map(|s| s.to_string_lossy().into()))
                )
            })?
            .opt_certificate(Some(attr.certificate.iter()))
            .with_context(|| format!("could not add certificate"))?
            .with_some(
                attr.accept_invalid_certs,
                ClientBuilder::danger_accept_invalid_certs,
            )
            .with_some(
                attr.accept_invalid_hostnames,
                ClientBuilder::danger_accept_invalid_hostnames,
            );

        Ok(builder)
    }
}

impl WithAttributes for RequestBuilder {
    fn with_attributes(self, attr: &Attributes) -> Result<Self> {
        let builder = self
            .with_some_result(attr.bearer_token_path.as_ref(), |builder, path| {
                let contents = read_to_string(path)?;
                Ok(builder.bearer_auth(contents))
            })?
            .with_some(attr.bearer_token.as_ref(), RequestBuilder::bearer_auth)
            .with_some_result(attr.basic_auth.as_ref(), |builder, path| {
                let contents = read_to_string(path)?;
                let mut parts = contents.splitn(2, ":");
                Ok(builder.basic_auth(parts.next().unwrap(), parts.next()))
            })?
            .with_some(attr.basic_auth.as_ref(), |b, basic_auth| {
                let mut parts = basic_auth.splitn(2, ":");
                b.basic_auth(parts.next().unwrap(), parts.next())
            })
            .opt_timeout(attr.timeout.as_ref())
            .with_context(|| format!("{:?} is not a valid format", attr.timeout.as_ref()))?
            .opt_version(attr.http_version.as_ref())
            .with_context(|| {
                format!(
                    "{:?} is not a valid http-version",
                    attr.http_version.as_ref()
                )
            })?;
        Ok(builder)
    }
}
