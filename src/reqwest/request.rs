use duration_string::DurationString;
use http::Version;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Request, RequestBuilder,
};
use std::str::FromStr;
use std::{collections::HashMap, fs, io, time::Duration};

use crate::{impl_opt, impl_when, Error, Expand, RenderGroup, Result};

#[derive(Debug, Clone)]
/// KeyValue enables you to turn a string like `key=value` into an actual key value
/// object.
pub struct KeyValue {
    /// name is the name of the key value
    name: String,
    /// value is the value of the key value
    value: String,
}

impl TryFrom<&String> for KeyValue {
    type Error = crate::Error;

    fn try_from(value: &String) -> Result<Self> {
        let mut parts = value.splitn(2, "=");

        let name: String = parts
            .next()
            .ok_or(crate::Error::from(format!(
                "{value} is not a valid key=value"
            )))?
            .trim()
            .into();
        let value: String = parts
            .next()
            .ok_or(crate::Error::from(format!(
                "{value} is not a valid key=value"
            )))?
            .trim()
            .into();

        Ok(KeyValue { name, value })
    }
}

/// This implementation allows for a template to be turned into a
/// key value object
impl<'a> TryFrom<RenderGroup<'a>> for KeyValue {
    type Error = crate::Error;

    fn try_from(value: RenderGroup<'a>) -> std::result::Result<Self, Self::Error> {
        let tmpl_output = value.render()?;

        let kv = KeyValue {
            name: value.name,
            value: tmpl_output,
        };
        Ok(kv)
    }
}

// This allows us to extend the reqwest RequestBuilder so that we can pass data from clap
// directly into it, creating a seamless interface. This implementation leaves the raw data
// within clap, and greatly reduces the number of copies needed.
pub trait KlaRequestBuilder {
    // opt_headers takes the headers from the `--header` argument and applies them to the
    // request being created.
    fn opt_headers<E, T, V>(self, headers: Option<T>) -> Result<RequestBuilder>
    where
        E: Into<Error>,
        V: TryInto<KeyValue, Error = E>,
        T: Iterator<Item = V>;

    fn opt_query<E, T, V>(self, headers: Option<T>) -> Result<RequestBuilder>
    where
        E: Into<Error>,
        V: TryInto<KeyValue, Error = E>,
        T: Iterator<Item = V>;

    fn opt_form<E, T, V>(self, form: Option<T>) -> Result<RequestBuilder>
    where
        E: Into<Error>,
        V: TryInto<KeyValue, Error = E>,
        T: Iterator<Item = V>;

    fn opt_body<'a>(self, body: Option<&String>) -> Result<RequestBuilder>;

    fn opt_basic_auth(self, userpass: Option<&String>) -> Result<RequestBuilder>;

    fn opt_bearer_auth(self, token: Option<&String>) -> Result<RequestBuilder>;

    fn opt_timeout(self, timeout: Option<&String>) -> Result<RequestBuilder>;

    fn opt_version(self, version: Option<&String>) -> Result<RequestBuilder>;
}

impl KlaRequestBuilder for RequestBuilder {
    fn opt_version(self, version: Option<&String>) -> Result<RequestBuilder> {
        if let None = version {
            return Ok(self);
        }

        let version = match version.unwrap().as_str() {
            "0.9" => Ok(Version::HTTP_09),
            "1.0" => Ok(Version::HTTP_10),
            "1.1" => Ok(Version::HTTP_11),
            "2.0" => Ok(Version::HTTP_2),
            "3.0" => Ok(Version::HTTP_3),
            _ => Err(Error::from("invalid http version")),
        }?;

        Ok(self.version(version))
    }

    fn opt_timeout(self, timeout: Option<&String>) -> Result<RequestBuilder> {
        if let None = timeout {
            return Ok(self);
        }

        // duration_string?!?!?!?! why do you return a string as an error
        // what the f**k is wrong with you.
        // Also thanks for the library!
        let d: Duration = match DurationString::from_str(timeout.unwrap()) {
            Ok(v) => Ok(v),
            Err(msg) => Err(Error::from(msg)),
        }?
        .into();

        Ok(self.timeout(d))
    }

    fn opt_basic_auth(self, userpass: Option<&String>) -> Result<RequestBuilder> {
        let userpass = if let Some(userpass) = userpass {
            match userpass.chars().nth(0) {
                Some('-') => io::read_to_string(io::stdin())?,
                Some('@') => fs::read_to_string(
                    userpass
                        .strip_prefix("@")
                        .expect("the prefix and the matching arm must match for this to work")
                        .shell_expansion(),
                )?,
                _ => userpass.into(),
            }
        } else {
            return Ok(self);
        };

        let mut parts = userpass.splitn(2, ":");
        Ok(self.basic_auth(parts.next().unwrap(), parts.next()))
    }

    fn opt_bearer_auth(self, token: Option<&String>) -> Result<RequestBuilder> {
        let token = if let Some(token) = token {
            match token.chars().nth(0) {
                Some('-') => io::read_to_string(io::stdin())?,
                Some('@') => fs::read_to_string(
                    token
                        .strip_prefix("@")
                        .expect("the prefix and the matching arm must match for this to work")
                        .shell_expansion(),
                )?,
                _ => token.into(),
            }
        } else {
            return Ok(self);
        };

        Ok(self.bearer_auth(token))
    }

    fn opt_body<'a>(self, body: Option<&String>) -> Result<RequestBuilder> {
        let body = if let Some(body) = body {
            match body.chars().nth(0) {
                Some('-') => io::read_to_string(io::stdin())?,
                Some('@') => fs::read_to_string(
                    body.strip_prefix("@")
                        .expect("the prefix and the matching arm must match for this to work")
                        .shell_expansion(),
                )?,
                _ => body.into(),
            }
        } else {
            return Ok(self);
        };

        Ok(self.body(body))
    }

    fn opt_query<E, T, V>(self, query: Option<T>) -> Result<RequestBuilder>
    where
        E: Into<Error>,
        V: TryInto<KeyValue, Error = E>,
        T: Iterator<Item = V>,
    {
        let query = if let Some(query) = query {
            query
        } else {
            return Ok(self);
        };

        let mut map = HashMap::new();

        for item in query {
            let item: KeyValue = item.try_into().map_err(|err| err.into())?;
            map.insert(item.name, item.value);
        }

        if map.is_empty() {
            Ok(self)
        } else {
            Ok(self.query(&map))
        }
    }

    fn opt_form<E, T, V>(self, form: Option<T>) -> Result<RequestBuilder>
    where
        E: Into<Error>,
        V: TryInto<KeyValue, Error = E>,
        T: Iterator<Item = V>,
    {
        let form = if let Some(form) = form {
            form
        } else {
            return Ok(self);
        };

        let mut map = HashMap::new();

        for item in form {
            let item: KeyValue = item.try_into().map_err(|err| err.into())?;
            map.insert(item.name, item.value);
        }

        if map.is_empty() {
            Ok(self)
        } else {
            Ok(self.form(&map))
        }
    }

    fn opt_headers<E, T, V>(self, headers: Option<T>) -> Result<RequestBuilder>
    where
        E: Into<Error>,
        V: TryInto<KeyValue, Error = E>,
        T: Iterator<Item = V>,
    {
        let headers = if let Some(headers) = headers {
            headers
        } else {
            return Ok(self);
        };

        let mut map = HeaderMap::new();

        for item in headers {
            let item: KeyValue = item.try_into().map_err(|err| err.into())?;
            map.insert(
                HeaderName::try_from(item.name)?,
                HeaderValue::try_from(item.value)?,
            );
        }

        if map.is_empty() {
            Ok(self)
        } else {
            Ok(self.headers(map))
        }
    }
}

impl_opt!(RequestBuilder, crate::Error);
impl_when!(Request);
