use std::{fmt::Debug, io::Cursor};

use crate::{impl_opt, impl_when, ContextBuilder, FetchMany, Result};
use reqwest::Response;
use tera::Tera;
use tokio::io::{AsyncRead, AsyncWrite};

// OutputBuilder collects all the info needed to render the output once
// kla has made the http request. (or reqwest rather)
pub struct OutputBuilder {
    // tmpl holds all the templates
    tmpl: Tera,
}

impl OutputBuilder {
    // new returns a new output builder. If left unchanged a call to render would
    // output nothing
    pub fn new() -> Self {
        OutputBuilder {
            tmpl: Tera::default(),
        }
    }

    // opt template sets the template
    pub fn template<S: AsRef<str>>(mut self, template: S) -> Result<Self> {
        self.tmpl.add_raw_template("body", template.as_ref())?;
        Ok(self)
    }

    // build creates the output
    pub async fn build(self, response: Response) -> Result<Output> {
        let OutputBuilder { tmpl } = self;

        // Write the body output
        let output = match tmpl.has("body") {
            true => {
                let buf = tmpl.render(
                    "body",
                    &ContextBuilder::new()
                        .insert_response(response)
                        .await?
                        .build(),
                )?;
                Output::from(buf)
            }
            false => Output::from(response),
        };

        Ok(output)
    }
}

#[derive(Debug)]
/// Output is the output of the template
pub enum Output {
    Steam(StreamingOutput),
    Owned(OwnedOuput),
}

/// Enable output to be built from StreamingOutput
impl From<StreamingOutput> for Output {
    fn from(value: StreamingOutput) -> Self {
        Output::Steam(value)
    }
}

/// Enable output to be built from OwnedOutput
impl From<OwnedOuput> for Output {
    fn from(value: OwnedOuput) -> Self {
        Output::Owned(value)
    }
}

/// Create an ownded output from a string
impl From<String> for Output {
    fn from(value: String) -> Self {
        Self::from(OwnedOuput::new(value))
    }
}

impl From<Response> for Output {
    fn from(value: Response) -> Self {
        Self::from(StreamingOutput { response: value })
    }
}

impl Output {
    pub async fn copy<'a, W>(self, writer: &'a mut W) -> tokio::io::Result<u64>
    where
        W: AsyncWrite + Unpin + ?Sized,
    {
        match self {
            Output::Steam(streaming_output) => {
                let mut reader = streaming_output.as_async_reader();
                tokio::io::copy(&mut reader, writer).await
            }
            Output::Owned(owned_ouput) => {
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
