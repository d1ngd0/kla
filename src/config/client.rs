use crate::{KlaClientBuilder, KlaRequestBuilder, Opt, Result};
use anyhow::Context;
use reqwest::{ClientBuilder, RequestBuilder};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Default)]
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

pub trait WithAttributes: Sized {
    /// with_attributes allows the attributes to be applied to any builders
    fn with_attributes(self, attr: &Attributes) -> Result<Self>;
}

impl WithAttributes for ClientBuilder {
    fn with_attributes(self, attr: &Attributes) -> Result<Self> {
        let builder = self
            .opt_header_agent(attr.agent.as_ref())
            .with_context(|| format!("could not add agent: {:?}", attr.agent.as_ref()))?
            .gzip(!attr.no_gzip.unwrap_or_default())
            .brotli(!attr.no_brotli.unwrap_or_default())
            .deflate(!attr.no_deflate.unwrap_or_default())
            .connection_verbose(attr.verbose.unwrap_or_default())
            .opt_connect_timeout(attr.connect_timeout.as_ref())?
            .opt_max_redirects(attr.max_redirects.as_ref())
            .no_redirects(attr.no_redirects.unwrap_or_default())
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
            .opt_bearer_auth(attr.bearer_token.as_ref())?
            .opt_basic_auth(attr.basic_auth.as_ref())?
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
