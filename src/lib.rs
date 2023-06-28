mod error;
mod optional_file;

pub use crate::error::Error;
pub use crate::optional_file::OptionalFile;

use config::Config;
use duration_string::DurationString;
use http::Version;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    redirect::Policy,
    Body, Client, ClientBuilder, Method, RequestBuilder,
};
use std::str::FromStr;
use std::{
    collections::HashMap,
    fs,
    io::{self, Read},
    time::Duration,
};
use tera::{Context, Tera};

pub trait KlaClientBuilder {
    fn opt_header_agent<'a>(self, agent: Option<&'a String>) -> Result<ClientBuilder, Error>;

    fn opt_max_redirects(self, redirects: Option<&usize>) -> ClientBuilder;

    fn no_redirects(self, no_redirects: bool) -> ClientBuilder;
}

impl KlaClientBuilder for ClientBuilder {
    fn no_redirects(self, no_redirects: bool) -> ClientBuilder {
        if no_redirects {
            self.redirect(Policy::none())
        } else {
            self
        }
    }

    fn opt_max_redirects(self, redirects: Option<&usize>) -> ClientBuilder {
        if let None = redirects {
            return self;
        }

        let redirects = redirects.unwrap();
        self.redirect(Policy::limited(*redirects))
    }

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

pub struct TemplateBuilder {
    template: Option<Tera>,
    failure_template: Option<Tera>,
    request: Option<RequestBuilder>,
    context: Option<Context>,
    output: Box<dyn std::io::Write>,
}

impl TemplateBuilder {
    pub fn new(output: Box<dyn std::io::Write>) -> TemplateBuilder {
        TemplateBuilder {
            template: None,
            failure_template: None,
            request: None,
            context: None,
            output,
        }
    }

    pub fn new_opt_file(path: Option<&String>) -> Result<TemplateBuilder, Error> {
        if let None = path {
            return Ok(TemplateBuilder::new_stdout());
        }

        Ok(TemplateBuilder::new_file(path.unwrap())?)
    }

    pub fn new_stdout() -> TemplateBuilder {
        TemplateBuilder {
            template: None,
            failure_template: None,
            request: None,
            context: None,
            output: Box::new(std::io::stdout()),
        }
    }

    pub fn new_file(path: &str) -> Result<TemplateBuilder, Error> {
        let file = std::fs::File::create(path)?;
        Ok(TemplateBuilder {
            template: None,
            failure_template: None,
            request: None,
            context: None,
            output: Box::new(file),
        })
    }

    pub fn new_buffer() -> TemplateBuilder {
        TemplateBuilder {
            template: None,
            failure_template: None,
            request: None,
            context: None,
            output: Box::new(std::io::Cursor::new(Vec::new())),
        }
    }

    fn parse_template(template: Option<&String>) -> Result<Tera, Error> {
        let mut tera = Tera::default();
        if let None = template {
            tera.add_raw_template("template", "{{ resp_body }}")?;
            return Ok(tera);
        }
        let template = template.unwrap();

        let mut chars = template.chars();
        match chars.next() {
            Some('@') => {
                let content = fs::read_to_string(chars.as_str())?;
                tera.add_raw_template("template", &content)
            }
            Some(_) => tera.add_raw_template("template", template),
            None => tera.add_raw_template("template", "{{ resp_body }}"),
        }?;

        Ok(tera)
    }

    pub fn opt_template(mut self, template: Option<&String>) -> Result<Self, Error> {
        self.template = Some(Self::parse_template(template)?);
        Ok(self)
    }

    pub fn opt_failure_template(mut self, template: Option<&String>) -> Result<Self, Error> {
        self.failure_template = Some(Self::parse_template(template)?);
        Ok(self)
    }

    pub fn request(mut self, request: RequestBuilder) -> Self {
        self.request = Some(request);
        self
    }

    pub fn build(self) -> Result<Template, Error> {
        Ok(Template {
            template: self.template,
            failure_template: self.failure_template,
            request: self.request.ok_or(Error::InvalidArguments(
                "you must supply a request".to_owned(),
            ))?,
            output: self.output,
            context: self.context.unwrap_or(Context::new()),
        })
    }
}

pub struct Template {
    template: Option<Tera>,
    failure_template: Option<Tera>,
    output: Box<dyn std::io::Write>,
    request: RequestBuilder,
    context: Context,
}

impl Template {
    pub async fn send(self) -> Result<(), Error> {
        let Template {
            template,
            failure_template,
            mut output,
            request,
            mut context,
        } = self;

        let response = request.send().await?;
        context.insert("resp_status", response.status().as_str());

        let headers = response.headers();
        for (name, value) in headers.iter() {
            context.insert(&format!("resp_headers_{}", name), &value.to_str()?);
        }

        let version = response.version();
        match version {
            Version::HTTP_09 => context.insert("resp_http_version", &"HTTP/0.9"),
            Version::HTTP_10 => context.insert("resp_http_version", &"HTTP/1.0"),
            Version::HTTP_11 => context.insert("resp_http_version", &"HTTP/1.1"),
            Version::HTTP_2 => context.insert("resp_http_version", &"HTTP/2.0"),
            Version::HTTP_3 => context.insert("resp_http_version", &"HTTP/3.0"),
            _ => context.insert("resp_http_version", &"Unknown"),
        }

        let template = if response.status().is_success() {
            &template
        } else {
            &failure_template
        };

        let content = response.text().await?;
        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(v) => context.extend(Context::from_value(v)?),
            _ => (),
        }
        context.insert("resp_body", &content);

        match template {
            None => output.write_all(content.as_bytes())?,
            Some(template) => template.render_to("template", &context, &mut output)?,
        }

        Ok(())
    }
}
