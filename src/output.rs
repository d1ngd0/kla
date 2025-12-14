use std::{fmt::Debug, io::Cursor};

use crate::{impl_opt, impl_when, ContextBuilder, FetchMany, Result};
use http::{HeaderMap, HeaderValue, StatusCode};
use reqwest::Response;
use tera::Tera;
use tokio::io::{AsyncRead, AsyncWrite};

// OutputBuilder collects all the info needed to render the output once
// kla has made the http request. (or reqwest rather)
pub struct OutputBuilder {
    // tmpl holds all the templates
    tmpl: Tera,
    desired_location: Option<String>,
}

impl OutputBuilder {
    // new returns a new output builder. If left unchanged a call to render would
    // output nothing
    pub fn new() -> Self {
        OutputBuilder {
            tmpl: Tera::default(),
            desired_location: None,
        }
    }

    // opt template sets the template
    pub fn template<S: AsRef<str>>(mut self, template: S) -> Result<Self> {
        self.tmpl.add_raw_template("body", template.as_ref())?;
        Ok(self)
    }

    // desired_location specifies where the user has specified they want the output
    // to be sent
    pub fn desired_location(mut self, output: String) -> Self {
        self.desired_location = Some(output);
        self
    }

    // build creates the output
    // TODO: We need to set context from arguments here as well so
    // args can manipulate the output template
    pub async fn build(self, response: Response) -> Result<Output> {
        let OutputBuilder {
            tmpl,
            desired_location,
        } = self;
        let headers = response.headers().clone();
        let status = response.status();

        // Write the body output
        let content = match tmpl.has("body") {
            true => {
                let buf = tmpl.render(
                    "body",
                    &ContextBuilder::new()
                        .insert_response(response)
                        .await?
                        .build(),
                )?;
                OutputContent::from(buf)
            }
            false => OutputContent::from(response),
        };

        Ok(Output {
            status,
            headers,
            desired_location,
            content,
        })
    }
}

#[derive(Debug)]
pub struct Output {
    status: StatusCode,
    headers: HeaderMap<HeaderValue>,
    desired_location: Option<String>,
    content: OutputContent,
}

impl Output {
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        &self.headers
    }

    pub fn desired_location(&self) -> Option<&String> {
        self.desired_location.as_ref()
    }

    pub async fn copy<'a, W>(self, writer: &'a mut W) -> tokio::io::Result<u64>
    where
        W: AsyncWrite + Unpin + ?Sized,
    {
        self.content.copy(writer).await
    }
}

#[derive(Debug)]
/// Output is the output of the template
pub enum OutputContent {
    Steam(StreamingOutput),
    Owned(OwnedOuput),
}

/// Enable output to be built from StreamingOutput
impl From<StreamingOutput> for OutputContent {
    fn from(value: StreamingOutput) -> Self {
        OutputContent::Steam(value)
    }
}

/// Enable output to be built from OwnedOutput
impl From<OwnedOuput> for OutputContent {
    fn from(value: OwnedOuput) -> Self {
        OutputContent::Owned(value)
    }
}

/// Create an ownded output from a string
impl From<String> for OutputContent {
    fn from(value: String) -> Self {
        Self::from(OwnedOuput::new(value))
    }
}

impl From<Response> for OutputContent {
    fn from(value: Response) -> Self {
        Self::from(StreamingOutput { response: value })
    }
}

impl OutputContent {
    pub async fn copy<'a, W>(self, writer: &'a mut W) -> tokio::io::Result<u64>
    where
        W: AsyncWrite + Unpin + ?Sized,
    {
        match self {
            OutputContent::Steam(streaming_output) => {
                let mut reader = streaming_output.as_async_reader();
                tokio::io::copy(&mut reader, writer).await
            }
            OutputContent::Owned(owned_ouput) => {
                let mut reader: Cursor<String> = owned_ouput.into();
                tokio::io::copy(&mut reader, writer).await
            }
        }
    }
}

/// StreamingOutput is one that holds onto an AsyncReader instead
/// of owning the value. This is so raw response bodies can stay on
/// the wire instead of having to be fully consumed before doing
/// anything with them
pub struct StreamingOutput {
    response: Response,
}

impl StreamingOutput {
    fn as_async_reader(self) -> impl AsyncRead {
        use futures::stream::TryStreamExt;

        let stream = self.response.bytes_stream().map_err(std::io::Error::other);
        tokio_util::io::StreamReader::new(stream)
    }
}

impl Debug for StreamingOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamingOutput")
            .field("response", &self.response)
            .finish()
    }
}

#[derive(Debug, Clone)]
/// Owned output is output which is owned by the client, this is used
/// in cases where a template is utilized to generate the output.
pub struct OwnedOuput {
    body: String,
}

impl OwnedOuput {
    /// new creates a new OwnedOutput with the read head at the start of the
    /// string
    fn new(s: String) -> OwnedOuput {
        OwnedOuput { body: s }
    }
}

impl From<OwnedOuput> for Cursor<String> {
    fn from(value: OwnedOuput) -> Self {
        Cursor::new(value.body)
    }
}

impl_when!(OutputBuilder);
impl_opt!(OutputBuilder, crate::Error);
