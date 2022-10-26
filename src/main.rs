use clap::Parser;
use reqwest::{ Method, Body, Client, Url, header::{ HeaderMap, HeaderValue } };
use http::method::InvalidMethod;
use url::ParseError;
use std::str::FromStr;

mod client;

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
    fn url(&self) -> Result<Url, ParseError> {
        if let Some(arg) = self.arg2.as_ref() {
            Url::parse(arg.as_str())
        } else if let Some(arg) = self.arg1.as_ref() {
            Url::parse(arg.as_str())
        } else {
            Url::parse("/")
        }
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = RootArgs::parse();

    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("application/json"));

    let client = Client::builder()
        .default_headers(headers)
        .build()
        .expect("could not build client");

    let mut builder = client.request(
        args.method().expect("could not build method"), 
        args.url().expect("could not build url"),
    );

    if let Some(body) = args.body() {
        builder = builder.body(body);
    }

    let content = builder.send().await?.text().await?;
    print!("{:?}", content);
    Ok(())
}
