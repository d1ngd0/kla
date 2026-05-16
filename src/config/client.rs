use std::path::Path;

use crate::{clap::arg_file_value, KlaClientBuilder, KlaRequestBuilder, Opt, Result, When};
use anyhow::Context;
use reqwest::{redirect::Policy, ClientBuilder, RequestBuilder};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Attributes specify settings which can be configured
pub struct Attributes {
    agent: Option<String>,
    timeout: Option<String>,
    basic_auth: Option<String>,
    bearer_token: Option<String>,
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
    connect_timeout: Option<String>,
    // TODO: fix this so it isn't public
    pub sigv4: Option<bool>,
    pub sigv4_aws_profile: Option<String>,
    pub sigv4_aws_service: Option<String>,
    accept_invalid_certs: Option<bool>,
    accept_invalid_hostnames: Option<bool>,
    #[serde(default)]
    certificate: Vec<String>,
    verbose: Option<bool>,
}

impl Attributes {
    pub fn resolve_working_dir<P: AsRef<Path>>(&mut self, dir: P) {}
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
            .opt_proxy(attr.proxy.as_ref(), attr.proxy_auth.as_ref())
            .with_context(|| {
                format!(
                    "could not add proxy: --proxy={:?} --proxy-auth={:?}",
                    attr.proxy.as_ref(),
                    attr.proxy_auth.as_ref().map(|v| "*".repeat(v.len()))
                )
            })?
            .opt_proxy_http(attr.proxy_http.as_ref(), attr.proxy_auth.as_ref())
            .with_context(|| {
                format!(
                    "could not add proxy: --proxy-http={:?} --proxy-auth={:?}",
                    attr.proxy_http.as_ref(),
                    attr.proxy_auth.as_ref().map(|v| "*".repeat(v.len()))
                )
            })?
            .opt_proxy_https(attr.proxy_https.as_ref(), attr.proxy_auth.as_ref())
            .with_context(|| {
                format!(
                    "could not add proxy: --proxy-https={:?} --proxy-auth={:?}",
                    attr.proxy_https.as_ref(),
                    attr.proxy_auth.as_ref().map(|v| "*".repeat(v.len()))
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
            .with_some(
                arg_file_value(attr.bearer_token.as_ref(), "bearer_token")?,
                RequestBuilder::bearer_auth,
            )
            .with_some(
                arg_file_value(attr.basic_auth.as_ref(), "basic_auth")?,
                |b, basic_auth| {
                    let mut parts = basic_auth.splitn(2, ":");
                    b.basic_auth(parts.next().unwrap(), parts.next())
                },
            )
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
