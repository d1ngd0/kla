use std::path::{Path, PathBuf};

use crate::config::Authentication;
use clap::{arg, ArgAction, ArgMatches, Command};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Attributes specify settings which can be configured
pub struct Attributes {
    pub agent: Option<String>,
    pub timeout: Option<String>,
    pub bearer_token: Option<String>,
    pub bearer_token_path: Option<PathBuf>,
    pub http_version: Option<String>,
    #[serde(default)]
    pub no_gzip: bool,
    #[serde(default)]
    pub no_brotli: bool,
    #[serde(default)]
    pub no_deflate: bool,
    pub max_redirects: Option<usize>,
    #[serde(default)]
    pub no_redirects: bool,
    pub proxy: Option<String>,
    pub proxy_http: Option<String>,
    pub proxy_https: Option<String>,
    pub proxy_auth: Option<String>,
    pub proxy_auth_path: Option<PathBuf>,
    pub connect_timeout: Option<String>,
    #[serde(default)]
    pub accept_invalid_certs: bool,
    #[serde(default)]
    pub accept_invalid_hostnames: bool,
    pub auth: Option<Authentication>,
    #[serde(default)]
    pub verbose: bool,
    #[serde(default)]
    pub certificate: Vec<PathBuf>,
}

impl Attributes {
    pub fn clap_flags(cmd: Command) -> Command {
        cmd
        .arg(arg!(--agent <AGENT> "The header agent string"))
        .arg(arg!(--timeout <SECONDS> "The amount of time allotted for the request to finish"))
        .arg(arg!(--"basic-auth" <BASIC_AUTH> "The username and password seperated by :, a preceding @ denotes a file path."))
        .arg(arg!(--"bearer-token" <BEARER_TOKEN> "The bearer token to use in requests. A preceding @ denotes a file path."))
        .arg(arg!(--"http-version" <HTTP_VERSION> "The version of http to send the request as").value_parser(["0.9", "1.0", "1.1", "2.0", "3.0"]))
        .arg(arg!(--"no-gzip" "Do not automatically uncompress gzip responses").action(ArgAction::SetTrue))
        .arg(arg!(--"no-brotli" "Do not automatically uncompress brotli responses").action(ArgAction::SetTrue))
        .arg(arg!(--"no-deflate" "Do not automatically uncompress deflate responses").action(ArgAction::SetTrue))
        .arg(arg!(--"max-redirects" <NUMBER> "The number of redirects allowed"))
        .arg(arg!(--"no-redirects" "Disable any redirects").action(ArgAction::SetTrue))
        .arg(arg!(--proxy <PROXY> "The proxy to use for all requests."))
        .arg(arg!(--"proxy-http" <PROXY_HTTP> "The proxy to use for http requests."))
        .arg(arg!(--"proxy-https" <PROXY_HTTPS> "The proxy to use for https requests."))
        .arg(arg!(--"proxy-auth" <PROXY_AUTH> "The username and password seperated by :."))
        .arg(arg!(--"proxy-auth-file" <PROXY_AUTH> "The username and password seperated by : stored within a file"))
        .arg(arg!(--"connect-timeout" <DURATION> "The amount of time to allow for connection"))
        .arg(arg!(--"sigv4" "Sign the request with AWS v4 Signature").action(ArgAction::SetTrue))
        .arg(arg!(--"sigv4-aws-profile" <AWS_PROFILE> "The AWS profile to use when signing a request"))
        .arg(arg!(--"sigv4-service" <SERVICE> "The AWS Service to use when signing the request"))
        .arg(arg!(--"accept-invalid-certs" "Controls the use of certificate validation.").action(ArgAction::SetTrue).long_help("Warning

You should think very carefully before using this method. If invalid certificates are trusted, any certificate for any site will be trusted for use. This includes expired certificates. This introduces significant vulnerabilities, and should only be used as a last resort."))
        .arg(arg!(--"accept-invalid-hostnames" "Controls the use of hostname verification.").action(ArgAction::SetTrue).long_help("Warning

You should think very carefully before you use this method. If hostname verification is not used, any valid certificate for any site will be trusted for use from any other. This introduces a significant vulnerability to man-in-the-middle attacks."))
        .arg(arg!(--certificate <CERTIFICATE_FILE> "The path to the certificate to use for requests. Accepts PEM and DER, expects files to end in .der or .pem. defaults to pem").action(ArgAction::Append))
    }

    // merge applies values in the secondary to anything not in the primary and returns
    // a new Attributes type
    pub fn merge(&self, secondary: &Self) -> Self {
        let s = Self {
            agent: self.agent.clone().or_else(|| secondary.agent.clone()),
            timeout: self.timeout.clone().or_else(|| secondary.timeout.clone()),
            bearer_token: self
                .bearer_token
                .clone()
                .or_else(|| secondary.bearer_token.clone()),
            bearer_token_path: self
                .bearer_token_path
                .clone()
                .or_else(|| secondary.bearer_token_path.clone()),
            http_version: self
                .http_version
                .clone()
                .or_else(|| secondary.http_version.clone()),
            no_gzip: self.no_gzip || secondary.no_gzip,
            no_brotli: self.no_brotli || secondary.no_brotli,
            no_deflate: self.no_deflate || secondary.no_deflate,
            max_redirects: self.max_redirects.or(secondary.max_redirects),
            no_redirects: self.no_redirects || secondary.no_redirects,
            proxy: self.proxy.clone().or_else(|| secondary.proxy.clone()),
            proxy_http: self
                .proxy_http
                .clone()
                .or_else(|| secondary.proxy_http.clone()),
            proxy_https: self
                .proxy_https
                .clone()
                .or_else(|| secondary.proxy_https.clone()),
            proxy_auth: self
                .proxy_auth
                .clone()
                .or_else(|| secondary.proxy_auth.clone()),
            proxy_auth_path: self
                .proxy_auth_path
                .clone()
                .or_else(|| secondary.proxy_auth_path.clone()),
            connect_timeout: self
                .connect_timeout
                .clone()
                .or_else(|| secondary.connect_timeout.clone()),
            accept_invalid_certs: self.accept_invalid_certs || secondary.accept_invalid_certs,
            accept_invalid_hostnames: self.accept_invalid_hostnames
                || secondary.accept_invalid_hostnames,
            certificate: self
                .certificate
                .clone()
                .into_iter()
                .chain(secondary.certificate.clone().into_iter())
                .collect(),
            auth: self.auth.clone().or_else(|| secondary.auth.clone()),
            verbose: self.verbose || secondary.verbose,
        };
        log::debug!("Attributes Merged {:?}", s);
        s
    }
}

impl From<&ArgMatches> for Attributes {
    fn from(args: &ArgMatches) -> Self {
        Self {
            agent: args.get_one::<String>("agent").map(String::from),
            timeout: args.get_one::<String>("timeout").map(String::from),
            // TODO: These should be fixed once this is part of the environment
            bearer_token: None,
            bearer_token_path: None,
            http_version: args.get_one::<String>("http-version").map(String::from),
            no_gzip: args
                .get_one::<bool>("no-gzip")
                .map(|f| *f)
                .unwrap_or_default(),
            no_brotli: args
                .get_one::<bool>("no-brotli")
                .map(|f| *f)
                .unwrap_or_default(),
            no_deflate: args
                .get_one::<bool>("no-deflate")
                .map(|f| *f)
                .unwrap_or_default(),
            max_redirects: args.get_one::<usize>("max-redirects").map(|f| *f),
            no_redirects: args
                .get_one::<bool>("no-redirects")
                .map(|f| *f)
                .unwrap_or_default(),
            proxy: args.get_one::<String>("proxy").map(String::from),
            proxy_http: args.get_one::<String>("proxy-http").map(String::from),
            proxy_https: args.get_one::<String>("proxy-https").map(String::from),
            proxy_auth: args.get_one::<String>("proxy-auth").map(String::from),
            proxy_auth_path: args.get_one::<String>("proxy-auth-file").map(PathBuf::from),
            connect_timeout: args.get_one::<String>("timeout").map(String::from),
            accept_invalid_certs: args
                .get_one::<bool>("accept-invalid-certs")
                .map(|f| *f)
                .unwrap_or_default(),
            accept_invalid_hostnames: args
                .get_one::<bool>("accept-invalid-hostnames")
                .map(|f| *f)
                .unwrap_or_default(),
            certificate: args
                .get_many::<String>("certificate")
                .unwrap_or_default()
                .map(PathBuf::from)
                .collect(),
            // TODO: Authentication methods should all be flaggable
            auth: None,
            verbose: args.get_count("verbose") > 0,
        }
    }
}

impl Attributes {
    pub fn resolve_working_dir<P: AsRef<Path>>(&mut self, dir: P) {
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

        self.auth
            .as_mut()
            .map(|f| f.resolve_working_dir(dir.as_ref()));
    }
}

/// returns a pointer to the Attributes object
impl AsRef<Attributes> for Attributes {
    fn as_ref(&self) -> &Attributes {
        self
    }
}

/// returns a mut pointer to the Attributes object
impl AsMut<Attributes> for Attributes {
    fn as_mut(&mut self) -> &mut Attributes {
        self
    }
}
