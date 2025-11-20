use std::pin::Pin;

use crate::{impl_opt, impl_when, ContextBuilder, Expand, FetchMany, Result};
use reqwest::{Request, Response};
use tera::Tera;
use tokio::{
    fs::File,
    io::{stdout, AsyncWriteExt},
};

// OutputBuilder collects all the info needed to render the output once
// kla has made the http request. (or reqwest rather)
pub struct OutputBuilder {
    // tmpl holds all the templates
    tmpl: Tera,
    prelude: Vec<String>,

    // output
    prelude_output: Option<Pin<Box<dyn tokio::io::AsyncWrite>>>,
    output: Pin<Box<dyn tokio::io::AsyncWrite>>,
}

impl OutputBuilder {
    // new returns a new output builder. If left unchanged a call to render would
    // output nothing
    pub fn new() -> Self {
        OutputBuilder {
            output: Box::pin(stdout()),
            prelude_output: None,
            tmpl: Tera::default(),
            prelude: vec![],
        }
    }

    /// opt_output takes a command line argument and turns it into an output.
    /// the value Some(`-`) will output to standard out as will None
    /// any value passed is interpreted as a file path, and a new file
    /// is created.
    /// This output is used as the output location of the main body or template
    /// output of the request. Defaults to standard out
    pub async fn opt_output(mut self, output: Option<&String>) -> Result<Self> {
        if let Some(output) = output.map(|v| v.as_str()) {
            self.output = match output {
                "-" => Box::pin(stdout()),
                _ => Box::pin(File::create(output.shell_expansion()).await?),
            };
        }

        Ok(self)
    }

    /// opt_prelude_output takes a command line argument and turns it into an output.
    /// the value Some(`-`) will output to standard out as will None
    /// any value passed is interpreted as a file path, and a new file
    /// is created
    /// This output is used as the location of the prelude (Headers, etc) Defaults to
    /// main body output
    pub async fn opt_prelude_output(mut self, output: Option<&String>) -> Result<Self> {
        if let Some(prelude_output) = output.map(|v| v.as_str()) {
            self.prelude_output = Some(match prelude_output {
                "-" => Box::pin(stdout()),
                _ => Box::pin(File::create(prelude_output.shell_expansion()).await?),
            });
        }

        Ok(self)
    }

    // output sets the output of kla. This defaults to standard out
    pub fn output(mut self, output: Pin<Box<dyn tokio::io::AsyncWrite>>) -> Self {
        self.output = output;
        self
    }

    pub fn request_prelude(self, req: &Request) -> Self {
        self.request_version_prelude(req)
            .method_prelude(req)
            .url_prelude(req)
            .request_header_prelude(req)
            .body_prelude(req)
    }

    // header_prelude adds a header to the prelude
    pub fn request_header_prelude(mut self, req: &Request) -> Self {
        let mut buf = String::from("Request Headers\n");

        for (key, val) in req.headers() {
            buf.push_str(format!("\t{}: {:?}\n", key.as_str(), val).as_str());
        }
        self.prelude.push(buf);
        self
    }

    pub fn url_prelude(mut self, req: &Request) -> Self {
        self.prelude.push(format!("URL: {}", req.url()));
        self
    }

    pub fn method_prelude(mut self, req: &Request) -> Self {
        self.prelude.push(format!("{}", req.method()));
        self
    }

    pub fn body_prelude(mut self, req: &Request) -> Self {
        if let Some(b) = req.body() {
            match b.as_bytes().map(|v| str::from_utf8(v)) {
                Some(Ok(s)) => self.prelude.push(format!("Request Body:\n{}", s)),
                Some(Err(_)) => self.prelude.push("Request Body: binary body".into()),
                None => (),
            };
        }
        self
    }

    pub fn request_version_prelude(mut self, req: &Request) -> Self {
        self.prelude
            .push(format!("Request Version: {:?}", req.version()));
        self
    }

    /// response_prelude adds prelude stuff from the response payload
    pub fn response_prelude(self, resp: &Response) -> Self {
        self.response_header_prelude(resp)
            .code_prelude(resp)
            .response_version_prelude(resp)
    }

    // header_prelude adds a header to the prelude
    pub fn response_header_prelude(mut self, resp: &Response) -> Self {
        let mut buf = String::from("Response Headers\n");

        for (key, val) in resp.headers() {
            buf.push_str(format!("\t{}: {:?}\n", key.as_str(), val).as_str());
        }
        self.prelude.push(buf);
        self
    }

    // header_prelude adds a header to the prelude
    pub fn code_prelude(mut self, resp: &Response) -> Self {
        self.prelude.push(format!("{}", resp.status()));
        self
    }

    // header_prelude adds a header to the prelude
    pub fn response_version_prelude(mut self, resp: &Response) -> Self {
        self.prelude
            .push(format!("Response Version: {:?}", resp.version()));
        self
    }

    // output sets the output of kla. This defaults to standard out
    pub fn prelude_output(mut self, output: Pin<Box<dyn tokio::io::AsyncWrite>>) -> Self {
        self.prelude_output = Some(output);
        self
    }

    // opt template sets the template
    pub fn opt_template(mut self, template: Option<&String>) -> Result<Self> {
        let template = match template {
            Some(template) => template,
            None => return Ok(self),
        };

        // TODO: Add ability to reference files or standard input
        self.tmpl.add_raw_template("body", template)?;
        Ok(self)
    }

    // build creates the output
    pub async fn render(self, response: Response) -> Result<()> {
        let mut response = response;
        let OutputBuilder {
            tmpl,
            mut prelude_output,
            mut output,
            prelude,
        } = self;

        let prelude_output = prelude_output.as_mut().unwrap_or(&mut output);
        for item in prelude {
            let lines = item.split("\n");
            for line in lines {
                prelude_output.write_all("> ".as_bytes()).await?;
                prelude_output.write_all(line.as_bytes()).await?;
                prelude_output.write_all("\n".as_bytes()).await?;
            }
        }

        // Write the body output
        match tmpl.has("body") {
            true => {
                let buf = tmpl.render(
                    "body",
                    &ContextBuilder::new()
                        .insert_response(response)
                        .await?
                        .build(),
                )?;
                output.write_all(buf.as_bytes()).await?;
            }
            false => {
                while let Some(chunk) = response.chunk().await? {
                    output.write_all(chunk.as_ref()).await?;
                }
            }
        }

        Ok(())
    }
}

impl_when!(OutputBuilder);
impl_opt!(OutputBuilder, crate::Error);
