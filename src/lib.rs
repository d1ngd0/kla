mod error;
mod optional_file;
mod output_type;

pub use crate::error::Error;
pub use crate::optional_file::OptionalFile;
pub use crate::output_type::OutputType;

use config::Config;
use duration_string::DurationString;
use http::Version;
use reqwest::ClientBuilder;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Body, Client, Method, RequestBuilder,
};
use std::str::FromStr;
use std::{
    collections::HashMap,
    fs,
    io::{self, Read},
    time::Duration,
};

pub trait KlaClientBuilder {
    fn opt_header_agent<'a>(self, agent: Option<&'a String>) -> Result<ClientBuilder, Error>;
}

impl KlaClientBuilder for ClientBuilder {
    fn opt_header_agent<'a>(self, agent: Option<&'a String>) -> Result<ClientBuilder, Error> {
        if let None = agent {
            return Ok(self);
        }
        let agent = HeaderValue::from_str(agent.unwrap())?;
        Ok(self.user_agent(agent))
    }
}

pub trait KlaClient {
    // args allows us to pass the raw arguments into the builder, which work as follows
    //
    //   if no arguments are supplied, we are going to make the request against the uri /
    //
    //   if one argument is supplied, it will be assigned to the uri, and the method will
    //   be assumed to be GET.
    //
    //   if two arguments are supplied, they are assumed to be first the method, then the
    //   uri.
    //
    //   if three arguments are supplied, they are assumed to be first the method, then
    //   the uri, and finally the body. If the body begins with an @ it is assumed to be a
    //   path to a file.
    fn args<'a, T>(self, args: Option<T>, prefix: Option<&String>) -> Result<RequestBuilder, Error>
    where
        T: Iterator<Item = &'a String>;
}

impl KlaClient for Client {
    fn args<'a, T>(self, args: Option<T>, prefix: Option<&String>) -> Result<RequestBuilder, Error>
    where
        T: Iterator<Item = &'a String>,
    {
        if let None = args {
            return Ok(self.request(Method::GET, "/"));
        }

        let mut args = args.unwrap().map(|v| &v[..]);

        let mut uri = args.next().unwrap_or("/");

        let method = if let Some(arg2) = args.next() {
            let method = Method::from_bytes(uri.to_uppercase().as_bytes());
            uri = arg2;
            method?
        } else {
            Method::GET
        };

        let mut url;
        if let Some(prefix) = prefix {
            url = String::from(prefix.trim_end_matches("/"));
            url.push_str(uri);
        } else {
            url = String::from(uri)
        }

        let builder = self.request(method, url);

        if let Some(body) = args.next() {
            return builder.opt_body(Some(body));
        }

        Ok(builder)
    }
}

// This allows us to extend the reqwest RequestBuilder so that we can pass data from clap
// directly into it, creating a seamless interface. This implementation leaves the raw data
// within clap, and greatly reduces the number of copies needed.
pub trait KlaRequestBuilder {
    // opt_headers takes the headers from the `--header` argument and applies them to the
    // request being created.
    fn opt_headers<'a, T>(self, headers: Option<T>) -> Result<RequestBuilder, Error>
    where
        T: Iterator<Item = &'a String>;

    fn opt_query<'a, T>(self, headers: Option<T>) -> Result<RequestBuilder, Error>
    where
        T: Iterator<Item = &'a String>;

    fn opt_form<'a, T>(self, form: Option<T>) -> Result<RequestBuilder, Error>
    where
        T: Iterator<Item = &'a String>;

    fn opt_body<'a>(self, body: Option<&str>) -> Result<RequestBuilder, Error>;

    fn opt_basic_auth(self, userpass: Option<&String>) -> RequestBuilder;

    fn opt_bearer_auth(self, token: Option<&String>) -> RequestBuilder;

    fn opt_timeout(self, timeout: Option<&String>) -> Result<RequestBuilder, Error>;

    fn opt_version(self, version: Option<&String>) -> Result<RequestBuilder, Error>;
}

impl KlaRequestBuilder for RequestBuilder {
    fn opt_version(self, version: Option<&String>) -> Result<RequestBuilder, Error> {
        if let None = version {
            return Ok(self);
        }

        let version = match version.unwrap().as_str() {
            "0.9" => Ok(Version::HTTP_09),
            "1.0" => Ok(Version::HTTP_10),
            "1.1" => Ok(Version::HTTP_11),
            "2.0" => Ok(Version::HTTP_2),
            "3.0" => Ok(Version::HTTP_3),
            _ => Err(Error::InvalidArguments(String::from(
                "invalid http version",
            ))),
        }?;

        Ok(self.version(version))
    }

    fn opt_timeout(self, timeout: Option<&String>) -> Result<RequestBuilder, Error> {
        if let None = timeout {
            return Ok(self);
        }

        // duration_string?!?!?!?! why do you return a string as an error
        // what the f**k is wrong with you.
        // Also thanks for the library!
        let d: Duration = match DurationString::from_str(timeout.unwrap()) {
            Ok(v) => Ok(v),
            Err(msg) => Err(Error::InvalidArguments(msg)),
        }?
        .into();

        Ok(self.timeout(d))
    }

    fn opt_basic_auth(self, userpass: Option<&String>) -> RequestBuilder {
        if let None = userpass {
            return self;
        }
        let userpass = userpass.unwrap();
        let mut parts = userpass.splitn(2, ":");
        self.basic_auth(parts.next().unwrap(), parts.next())
    }

    fn opt_bearer_auth(self, token: Option<&String>) -> RequestBuilder {
        if let None = token {
            return self;
        }

        self.bearer_auth(token.unwrap())
    }

    fn opt_body<'a>(self, body: Option<&'a str>) -> Result<RequestBuilder, Error> {
        if let None = body {
            return Ok(self);
        }
        let body = body.unwrap();

        let mut body_chars = body.chars();

        let body = match body_chars.next() {
            Some('@') => {
                let name = body_chars.collect::<String>();
                Some(Body::from(fs::read_to_string(name)?))
            }
            Some('-') => {
                let mut buf = String::new();
                io::stdin().read_to_string(&mut buf)?;
                Some(Body::from(buf))
            }
            Some(_) => Some(Body::from(body.to_owned())),
            None => None,
        }
        .ok_or(Error::InvalidArguments("you must supply a body".to_owned()))?;

        Ok(self.body(body))
    }

    fn opt_query<'a, T>(self, query: Option<T>) -> Result<RequestBuilder, Error>
    where
        T: Iterator<Item = &'a String>,
    {
        if let None = query {
            return Ok(self);
        }

        let mut map = HashMap::new();
        query
            .unwrap()
            .map(|q| {
                let mut key_val = q.splitn(2, "=");
                let name = key_val
                    .next()
                    .ok_or(Error::InvalidArguments(format!(
                        "{q} is not a valid key=value"
                    )))?
                    .trim();
                let value = key_val
                    .next()
                    .ok_or(Error::InvalidArguments(format!(
                        "{q} is not a valid key=value"
                    )))?
                    .trim();

                map.insert(name, value);

                Ok(())
            })
            .collect::<Result<(), Error>>()?;

        Ok(self.query(&map))
    }

    fn opt_form<'a, T>(self, form: Option<T>) -> Result<RequestBuilder, Error>
    where
        T: Iterator<Item = &'a String>,
    {
        if let None = form {
            return Ok(self);
        }

        let mut map = HashMap::new();
        form.unwrap()
            .map(|formval| {
                let mut key_val = formval.splitn(2, "=");
                let name = key_val
                    .next()
                    .ok_or(Error::InvalidArguments(format!(
                        "{formval} is not a valid key=value"
                    )))?
                    .trim();
                let value = key_val
                    .next()
                    .ok_or(Error::InvalidArguments(format!(
                        "{formval} is not a valid key=value"
                    )))?
                    .trim();

                map.insert(name, value);

                Ok(())
            })
            .collect::<Result<(), Error>>()?;

        Ok(self.form(&map))
    }

    fn opt_headers<'a, T>(self, headers: Option<T>) -> Result<RequestBuilder, Error>
    where
        T: Iterator<Item = &'a String>,
    {
        if let None = headers {
            return Ok(self);
        }

        let mut map = HeaderMap::new();
        headers
            .unwrap()
            .map(|header| {
                let mut key_val = header.splitn(2, ":");
                let name = key_val
                    .next()
                    .ok_or(Error::InvalidArguments(format!(
                        "{header} is not a valid http header"
                    )))?
                    .trim();
                let value = key_val
                    .next()
                    .ok_or(Error::InvalidArguments(format!(
                        "{header} is not a valid http header"
                    )))?
                    .trim();

                map.insert(
                    HeaderName::from_bytes(name.as_bytes())?,
                    HeaderValue::from_bytes(value.as_bytes())?,
                );

                Ok(())
            })
            .collect::<Result<(), Error>>()?;

        Ok(self.headers(map))
    }
}

pub fn environment(env: Option<&String>, config: &Config) -> Option<String> {
    if let None = env {
        return None;
    }
    let env = env.unwrap();

    let mut s = String::from("environments.");
    s.push_str(env);

    match config.get_string(&s) {
        Ok(v) => Some(v),
        Err(_) => None,
    }
}
