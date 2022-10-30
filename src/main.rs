use clap::Parser;
use http::method::InvalidMethod;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Body, Method, Url,
};
use std::str::FromStr;
use url::ParseError;

mod client;
use client::Client;

mod konfig;
use konfig::Config;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct RootArgs {
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
    arg1: Option<String>,

    arg2: Option<String>,

    arg3: Option<String>,

    #[arg(short, long)]
    env: Option<String>,
}

impl RootArgs {
    // url will return the url as supplied by the arguments
    fn url(&self, prefix: &str) -> Result<Url, ParseError> {
        let mut uri = "/";

        if let Some(arg) = self.arg2.as_ref() {
            uri = arg.as_str();
        } else if let Some(arg) = self.arg1.as_ref() {
            uri = arg.as_str();
        }

        let mut url = match prefix.strip_suffix("/") {
            Some(p) => String::from(p),
            None => String::from(prefix),
        };

        url.push_str(uri);

        Url::parse(url.as_str())
    }

    fn method(&self) -> Result<Method, InvalidMethod> {
        let method = if self.arg2.is_some() {
            self.arg1.as_ref().expect("how did you? what now?").as_str()
        } else {
            "GET"
        };

        Method::from_str(method.to_ascii_uppercase().as_str())
    }

    fn body(&self) -> Option<Body> {
        let body = self.arg3.as_ref()?;
        Some(Body::from(body.clone()))
    }

    fn env(&self) -> String {
        if let Some(s) = self.env.as_ref() {
            String::from(s)
        } else {
            String::from("")
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = RootArgs::parse();

    let conf = Config::new("config.toml")?;

    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("application/json"));

    let prefix = conf.prefix(args.env().as_str());
    let client = Client::new(headers)?;

    let content = client
        .send(args.method()?, args.url(prefix.as_str())?, args.body())
        .await?
        .text()
        .await?;

    print!("{:?}", content);
    Ok(())
}
