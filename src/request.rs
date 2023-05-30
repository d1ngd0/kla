use super::{authentication::AuthType, error::Error, output_type::OutputType};
use config::Config;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Body, Method, Url,
};
use serde_json::Value;
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    str::{self, FromStr},
};
use tera::{Context, Tera};

pub async fn request(mut args: RequestArgs) -> Result<(), Error> {
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let mut request = client.request(args.method, args.url);
    request = args.auth.apply(request);

    if let Some(body) = args.body {
        request = request.body(body);
    }

    let content = request.send().await?.text().await.unwrap();

    let bbody = str::from_utf8(content.as_bytes())?;
    let body = serde_json::from_str::<Value>(bbody);

    if let Err(_) = body {
        args.output.write(bbody.as_bytes())?;
        return Ok(());
    }

    let mut context = Context::new();
    context.insert("body", &body.unwrap());
    let res = args.template.render("_", &context)?;

    args.output.write(res.as_bytes())?;
    Ok(())
}

pub struct RequestArgs {
    method: Method,
    url: Url,
    body: Option<Body>,
    template: Tera,
    output: OutputType,
    auth: AuthType,
}

pub struct RequestArgsBuilder {
    method: Option<String>,
    uri: Option<String>,
    body: Option<String>,
    prefix: Option<String>,
    template: Option<String>,
    output: Option<String>,
    basic_auth: Option<String>,
    bearer_token: Option<String>,
}

impl RequestArgsBuilder {
    pub fn new() -> RequestArgsBuilder {
        RequestArgsBuilder {
            method: None,
            uri: None,
            body: None,
            prefix: None,
            template: None,
            output: None,
            basic_auth: None,
            bearer_token: None,
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

    pub fn output_file(mut self, path: &str) -> Result<RequestArgsBuilder, Error> {
        self.output = Some(String::from(path));
        Ok(self)
    }

    pub fn environment(mut self, env: &str, config: &Config) -> Result<RequestArgsBuilder, Error> {
        let mut s = String::from("environments.");
        s.push_str(env);

        let prefix = config.get_string(&s)?;

        self.prefix = Some(String::from(prefix.trim_end_matches("/")));

        Ok(self)
    }

    fn build_body(body: Option<String>) -> Result<Option<Body>, Error> {
        if let None = body {
            return Ok(None);
        }

        let body = body.unwrap();
        let mut body_chars = body.chars();

        match body_chars.next() {
            Some('@') => {
                let name = body_chars.collect::<String>();
                Ok(Some(Body::from(fs::read_to_string(name)?)))
            }
            Some('-') => {
                let mut buf = String::new();
                io::stdin().read_to_string(&mut buf)?;
                Ok(Some(Body::from(buf)))
            }
            Some(_) => Ok(Some(Body::from(body))),
            None => Ok(None),
        }
    }

    fn build_template(template: Option<String>) -> Result<Tera, Error> {
        let mut tera = Tera::default();

        if let None = template {
            tera.add_raw_template("_", "{{ body | json_encode(pretty=true) }}")?;
            return Ok(tera);
        }

        let template = template.unwrap();
        let mut template_chars = template.chars();

        match template_chars.next() {
            Some('@') => {
                let name: String = template_chars.collect::<String>();
                let content = fs::read_to_string(&name)?;
                tera.add_raw_template("_", &content)?;
            }
            Some(_) => tera.add_raw_template("_", &template)?,
            None => tera.add_raw_template("_", "{{ body | json_encode(pretty=true) }}")?,
        };

        Ok(tera)
    }

    fn build_output(path: Option<String>) -> Result<OutputType, Error> {
        match path {
            None => Ok(OutputType::StdOut),
            Some(path) => Ok(OutputType::File(Box::new(File::create(&path)?))),
        }
    }

    fn parse_bearer_token(token: String) -> Result<AuthType, Error> {
        let mut chars = token.chars();

        match chars.next() {
            Some('@') => {
                let name = chars.collect::<String>();
                Ok(AuthType::bearer_from_file(&name)?)
            }
            Some(_) => Ok(AuthType::bearer_from_string(token)),
            None => Err(Error::InvalidArguments(String::from(
                "no bearer token supplied",
            ))),
        }
    }

    fn parse_basic_auth(basic: String) -> Result<AuthType, Error> {
        let mut chars = basic.chars();

        match chars.next() {
            Some('@') => {
                let name = chars.collect::<String>();
                Ok(AuthType::basic_from_file(&name)?)
            }
            Some(_) => AuthType::basic_from_string(&basic),
            None => Err(Error::InvalidArguments(String::from(
                "no basic authentication supplied",
            ))),
        }
    }

    fn build_auth(
        bearer_token: Option<String>,
        basic_auth: Option<String>,
    ) -> Result<AuthType, Error> {
        if let Some(bearer_token) = bearer_token {
            RequestArgsBuilder::parse_bearer_token(bearer_token)
        } else if let Some(basic_auth) = basic_auth {
            RequestArgsBuilder::parse_basic_auth(basic_auth)
        } else {
            Ok(AuthType::None)
        }
    }

    pub fn output(mut self, output: String) -> RequestArgsBuilder {
        self.output = Some(output);
        self
    }

    pub fn template(mut self, template: Option<String>) -> RequestArgsBuilder {
        self.template = template;
        self
    }

    pub fn bearer_token(mut self, bearer_token: String) -> RequestArgsBuilder {
        self.bearer_token = Some(bearer_token);
        self
    }

    pub fn basic_auth(mut self, basic_auth: String) -> RequestArgsBuilder {
        self.basic_auth = Some(basic_auth);
        self
    }

    // build will build the thinger
    pub fn build(self) -> Result<RequestArgs, Error> {
        let RequestArgsBuilder {
            method,
            uri,
            body,
            prefix,
            template,
            output,
            basic_auth,
            bearer_token,
        } = self;

        let method = if let Some(method) = method {
            Method::from_str(&method)?
        } else {
            Method::GET
        };

        let mut url = uri.unwrap_or(String::from("/"));
        if let Some(prefix) = prefix {
            url.insert_str(0, &prefix)
        }

        Ok(RequestArgs {
            method,
            url: Url::parse(&url)?,
            body: RequestArgsBuilder::build_body(body)?,
            template: RequestArgsBuilder::build_template(template)?,
            output: RequestArgsBuilder::build_output(output)?,
            auth: RequestArgsBuilder::build_auth(basic_auth, bearer_token)?,
        })
    }
}
