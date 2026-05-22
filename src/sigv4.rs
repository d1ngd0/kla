use std::{fmt::Display, ops::Deref, str::FromStr, time::SystemTime};

use aws_config::BehaviorVersion;
use chrono::{DateTime, Utc};
use http::{
    header::{self, ToStrError},
    HeaderName, HeaderValue,
};
use reqwest::Request;

use anyhow::Context as _;
use aws_credential_types::{provider::ProvideCredentials, Credentials};
use aws_sigv4::{
    http_request::{
        sign, PayloadChecksumKind, SignableBody, SignableRequest,
        SigningError as Sigv4SigningError, SigningSettings,
    },
    sign::v4::{self, signing_params::BuildError},
};

use crate::{config, impl_opt, Authentication, Opt};

#[derive(thiserror::Error, Debug)]
/// SigningError will be returned from the builder when any issues arise
/// building the signature
pub enum SigningError {
    #[error("Could not build signature: {0}")]
    // BuildError is a general error, usually output because the caller
    // forgot some required field
    BuildError(String),

    #[error("could not build signature: {0}")]
    Sigv4Error(#[from] BuildError),
    #[error("could not build signature: {0}")]
    SigningError(#[from] Sigv4SigningError),
    #[error("could not turn header into string when signing: {0}")]
    ToStrError(#[from] ToStrError),
}

/// Implementing from allows you to call `SigningError::From("some string")`
/// how nice
impl From<&str> for SigningError {
    fn from(value: &str) -> Self {
        SigningError::BuildError(value.into())
    }
}

#[derive(Debug, Clone)]
/// Signed headers is a list of headers expected in the request we are
/// about to sign. `x-amz-date` will be specified for you based on the
/// date given to the builder (or date will be set to SystemTime::now
/// when left unspecified)
///
/// There are no values in this list, it is just the names of the headers
/// we expect. They should be added to this list the same way we see them
/// in the request. (amazon requires the signature to have them lowercase
/// we handle that for you.)
pub struct SignedHeaders(Vec<String>);

impl SignedHeaders {
    /// push will add an aditional header to the list
    fn push<S: Into<String>>(&mut self, value: S) {
        self.0.push(value.into());
    }
}

/// Create the default value with `host` and `x-amz-date` already there, since they are
/// required when creating a signature
impl Default for SignedHeaders {
    fn default() -> Self {
        Self(vec![String::from("host"), String::from("x-amz-date")])
    }
}

impl Deref for SignedHeaders {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// This implementation of display will turn the headers into a lowercase
/// ; delimited string, which is what we need in our Authorization.
impl Display for SignedHeaders {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let headers: Vec<String> = self.0.iter().map(|s| s.to_lowercase()).collect();
        write!(f, "{}", headers.join(";"))
    }
}

#[derive(Debug, Clone, Default)]
pub struct SigV4 {
    /// headers are the headers we want to sign
    headers: SignedHeaders,
    /// The date for the signature, if left unset it will default to
    /// right now
    date: Option<DateTime<Utc>>,
    /// region is the region where the request is being made
    region: Option<String>,
    /// service is the service we intend on making the request against
    service: Option<String>,
    /// credentials hold the AWS credentials for the builder
    credentials: Option<Credentials>,
}

impl Authentication for SigV4 {
    fn authorize(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> crate::Result<reqwest::RequestBuilder> {
        Ok(builder)
    }

    fn sign(&self, req: Request) -> crate::Result<Request> {
        Ok(SigV4::sign(&self, req)?)
    }
}

impl TryFrom<config::SigV4> for SigV4 {
    type Error = crate::Error;

    fn try_from(value: config::SigV4) -> Result<Self, Self::Error> {
        futures::executor::block_on(SigV4::new(value.profile.as_ref(), value.service.as_ref()))
    }
}

impl SigV4 {
    pub async fn new(profile: Option<&String>, service: Option<&String>) -> crate::Result<SigV4> {
        let config = aws_config::ConfigLoader::default()
            .behavior_version(BehaviorVersion::latest())
            .with_some(profile, aws_config::ConfigLoader::profile_name)
            .load()
            .await;

        let credentials = config
            .credentials_provider()
            .ok_or(anyhow::Error::msg("AWS credentials not found"))?
            .provide_credentials()
            .await
            .context("could not fetch credentials")?;

        let signer = Self::default()
            .date(SystemTime::now())
            .region(config.region().map(|r| r.to_string()).unwrap_or_default())
            .service(
                service
                    .map(|s| s.as_str())
                    .unwrap_or("execute-api")
                    .to_string(),
            )
            .credentials(credentials);

        Ok(signer)
    }

    /// header adds a new header to include when signing the request
    pub fn header<S: Into<String>>(mut self, header: S) -> Self {
        self.headers.push(header.into());
        self
    }

    /// date sets the date for the signature, if not set the default
    pub fn date(mut self, date: SystemTime) -> Self {
        self.date = Some(date.into());
        self
    }

    /// region sets the region for the signature
    pub fn region(mut self, region: String) -> Self {
        self.region = Some(region);
        self
    }

    /// service sets the service for the signature
    pub fn service(mut self, service: String) -> Self {
        self.service = Some(service);
        self
    }

    /// credentials sets the credentials for the signature
    pub fn credentials(mut self, credentials: Credentials) -> Self {
        self.credentials = Some(credentials);
        self
    }

    pub fn sign(&self, req: Request) -> Result<Request, SigningError> {
        let Self {
            headers,
            date,
            region,
            service,
            credentials,
        } = self;
        let mut req = req;

        // unwrap things we need or throw errors
        let credentials = credentials.clone().ok_or(SigningError::from(
            "aws credentials are required when creating sigv4 request",
        ))?;
        let date = date.unwrap_or(SystemTime::now().into());
        let region = region.as_ref().ok_or(SigningError::from(
            "region is required when creating a sigv4 request",
        ))?;
        let service = service.as_ref().ok_or(SigningError::from(
            "service is required when creating a sigv4 request",
        ))?;

        // make sure the host value is present
        if !req.headers().contains_key(header::HOST) {
            let host = req.url().host().map(|h| h.to_string()).unwrap_or_default();
            req.headers_mut().insert(
                header::HOST,
                HeaderValue::from_str(&host).expect("invalid header"),
            );
        }

        // we want to be using this format `YYYYMMDDTHHMMSSZ`
        // This makes sure the request has the x-amz-date applied to the
        // request. If the value was set we override it to make sure the
        // signature works.
        req.headers_mut().insert(
            "x-amz-date",
            HeaderValue::from_str(&format!("{}", date.format("%Y%m%d%H%M%SZ")))
                .expect("how can this be invalid"),
        );

        let identity = credentials.into();
        let mut signing_settings = SigningSettings::default();
        signing_settings.payload_checksum_kind = PayloadChecksumKind::XAmzSha256;

        let signing_params = v4::SigningParams::builder()
            .identity(&identity)
            .region(&region)
            .name(&service)
            .time(SystemTime::now())
            .settings(signing_settings)
            .build()?
            .into();

        // create the headers list from the paslisted headers
        let mut signed_headers: Vec<(&str, &str)> = vec![];
        for header in headers.iter() {
            signed_headers.push((
                header.as_str(),
                req.headers()
                    .get(header.as_str())
                    .ok_or(SigningError::BuildError(format!(
                        "missing header {} which is required for signing",
                        header
                    )))?
                    .to_str()?,
            ));
        }

        let signable_request = SignableRequest::new(
            req.method().as_str(),
            req.url().to_string(),
            signed_headers.into_iter(),
            SignableBody::Bytes(
                req.body()
                    .map(|m| m.as_bytes())
                    .flatten()
                    .unwrap_or_default(),
            ),
        )?;

        // Sign the request
        let (signing_instructions, _signature) =
            sign(signable_request, &signing_params)?.into_parts();
        let (headers, query) = signing_instructions.into_parts();

        // Add headers we need from signing
        for header in headers {
            req.headers_mut().insert(
                HeaderName::from_str(header.name()).expect("aws signature invalid"),
                HeaderValue::from_str(header.value()).expect("aws signature invalid"),
            );
        }

        // add query parameters needed from signing
        for param in query {
            req.url_mut()
                .query_pairs_mut()
                .append_pair(param.0, param.1.as_ref());
        }

        Ok(req)
    }
}

pub trait Sigv4Request {
    fn sign_request(
        self,
        profile: Option<&String>,
        service: Option<&String>,
    ) -> impl std::future::Future<Output = Result<Request, anyhow::Error>> + Send;
}

impl Sigv4Request for Request {
    async fn sign_request(
        self,
        profile: Option<&String>,
        service: Option<&String>,
    ) -> Result<Request, anyhow::Error> {
        let req = SigV4::new(profile, service)
            .await?
            .sign(self)
            .context("Could not sign request")?;

        Ok(req)
    }
}

impl_opt!(aws_config::ConfigLoader, crate::Error);
