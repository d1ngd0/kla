use http::method::InvalidMethod;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Body, Method, Url,
};
use serde_json::Value;
use std::str::{self, FromStr};
use tera::{Context, Tera};
use url::ParseError;

mod error;
mod klient;
mod konfig;

pub use crate::error::Error;
pub use crate::klient::Client;
pub use crate::konfig::Config;

pub async fn request(args: RequestArgs, conf: Config) -> Result<(), Error> {
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("application/json"));

    let prefix = conf.prefix(args.env().as_str());
    let client = Client::new(headers)?;
    let template = args.template("output")?;

    let content = client
        .send(args.method()?, args.url(prefix.as_str())?, args.body())
        .await?
        .text()
        .await
        .unwrap();

    let resp_body = str::from_utf8(content.as_bytes())?;

    match serde_json::from_str::<Value>(resp_body) {
        Err(_) => print!("{}", content),
        Ok(v) => {
            let mut context = Context::new();
            context.insert("body", &v);
            let res = template.render("output", &context)?;
            print!("{}", res)
        }
    }

    Ok(())
}

pub struct RequestArgs {
    method: Method,
    uri: String,
    body: Option<Body>,
    env: String,
    template: Tera,
}

pub struct RequestArgsBuilder {
    method: Option<String>,
    uri: Option<String>,
    body: Option<String>,
    env: Option<String>,
    template: Option<Tera>,
}

impl RequestArgsBuilder {
    pub fn new() -> RequestArgsBuilder {
        RequestArgsBuilder {
            method: None,
            uri: None,
            body: None,
            env: None,
            template: None,
        }
    }
    // arg 1 - 3 are to function in the following way
    // If only a single value is given, it will be the url.
    // The method is assumed to be GET and no bod
    //
    // 2 values given will result in the first value being
    // the method, and the second being the url.
    //
    // If all three are given, the first argument is the method
    // the second argument is the url and the final argument is
    // is the body.
    //
    // If an environment is supplied, it is prepended to the
    // start of the url.
    pub fn args(mut self, args: Vec<String>) -> Result<RequestArgsBuilder, Error> {
        let mut args = args.into_iter();
        if let Some(arg) = args.next() {
            self.uri = Some(arg)
        }

        if let Some(arg) = args.next() {
            self.method = self.uri;
            self.uri = Some(arg)
        }

        if let Some(arg) = args.next() {
            self.body = Some(arg)
        }

        if let Some(_) = args.next() {
            return Err(Error::InvalidArguments(String::from(
                "Additional arguments, you only need to pass 3",
            )));
        }

        Ok(self)
    }

    // build will build the thinger
    pub fn build(self) -> RequestArgs {}
}
