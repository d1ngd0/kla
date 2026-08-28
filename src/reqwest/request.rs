use http::Version;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Request, RequestBuilder,
};
use std::{collections::HashMap, time::Duration};
use std::{fmt::Display, str::from_utf8};

use crate::{clap::edit_value, impl_opt, Error, KResult, RenderGroup};

#[derive(Debug, Clone)]
/// KeyValue enables you to turn a string like `key=value` into an actual key value
/// object.
pub struct KeyValue<const SEP: char> {
    /// name is the name of the key value
    name: String,
    /// value is the value of the key value
    value: String,
}

impl<const SEP: char> Display for KeyValue<SEP> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}", self.name, SEP, self.value)
    }
}

impl<const SEP: char> TryFrom<&String> for KeyValue<SEP> {
    type Error = crate::Error;

    fn try_from(value: &String) -> KResult<Self> {
        let mut parts = value.splitn(2, SEP);

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
impl<'a, const SEP: char> TryFrom<RenderGroup<'a>> for KeyValue<SEP> {
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
    fn opt_headers<E, T, V>(self, headers: Option<T>) -> KResult<RequestBuilder>
    where
        E: Into<Error>,
        V: TryInto<KeyValue<':'>, Error = E>,
        T: Iterator<Item = V>;

    fn opt_query<E, T, V>(self, headers: Option<T>) -> KResult<RequestBuilder>
    where
        E: Into<Error>,
        V: TryInto<KeyValue<'='>, Error = E>,
        T: Iterator<Item = V>;

    fn opt_form<E, T, V>(self, form: Option<T>) -> KResult<RequestBuilder>
    where
        E: Into<Error>,
        V: TryInto<KeyValue<'='>, Error = E>,
        T: Iterator<Item = V>;

    fn opt_timeout(self, timeout: Option<&String>) -> KResult<RequestBuilder>;

    fn opt_version(self, version: Option<&String>) -> KResult<RequestBuilder>;
}

impl KlaRequestBuilder for RequestBuilder {
    fn opt_version(self, version: Option<&String>) -> KResult<RequestBuilder> {
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

    fn opt_timeout(self, timeout: Option<&String>) -> KResult<RequestBuilder> {
        if let None = timeout {
            return Ok(self);
        }

        // duration_string?!?!?!?! why do you return a string as an error
        // what the f**k is wrong with you.
        // Also thanks for the library!
        let d: Duration = match duration_str::parse(timeout.unwrap()) {
            Ok(v) => Ok(v),
            Err(msg) => Err(Error::from(msg)),
        }?
        .into();

        Ok(self.timeout(d))
    }

    fn opt_query<E, T, V>(self, query: Option<T>) -> KResult<RequestBuilder>
    where
        E: Into<Error>,
        V: TryInto<KeyValue<'='>, Error = E>,
        T: Iterator<Item = V>,
    {
        let query = if let Some(query) = query {
            query
        } else {
            return Ok(self);
        };

        let mut map = HashMap::new();

        for item in query {
            let item: KeyValue<'='> = item.try_into().map_err(|err| err.into())?;
            map.insert(item.name, item.value);
        }

        if map.is_empty() {
            Ok(self)
        } else {
            Ok(self.query(&map))
        }
    }

    fn opt_form<E, T, V>(self, form: Option<T>) -> KResult<RequestBuilder>
    where
        E: Into<Error>,
        V: TryInto<KeyValue<'='>, Error = E>,
        T: Iterator<Item = V>,
    {
        let form = if let Some(form) = form {
            form
        } else {
            return Ok(self);
        };

        let mut map = HashMap::new();

        for item in form {
            let item: KeyValue<'='> = item.try_into().map_err(|err| err.into())?;
            map.insert(item.name, item.value);
        }

        if map.is_empty() {
            Ok(self)
        } else {
            Ok(self.form(&map))
        }
    }

    fn opt_headers<E, T, V>(self, headers: Option<T>) -> KResult<RequestBuilder>
    where
        E: Into<Error>,
        V: TryInto<KeyValue<':'>, Error = E>,
        T: Iterator<Item = V>,
    {
        let headers = if let Some(headers) = headers {
            headers
        } else {
            return Ok(self);
        };

        let mut map = HeaderMap::new();

        for item in headers {
            let item: KeyValue<':'> = item.try_into().map_err(|err| err.into())?;
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

pub trait KlaRequest: Sized {
    #[allow(async_fn_in_trait)]
    async fn edit(self, edit: Option<&bool>) -> KResult<Self>;
}

impl KlaRequest for Request {
    async fn edit(mut self, edit: Option<&bool>) -> KResult<Self> {
        // if we aren't editing return right away
        if !edit.copied().unwrap_or_default() {
            return Ok(self);
        }

        let body = self.body_mut().take();
        let body = body
            .as_ref()
            .map(|b| b.as_bytes())
            .flatten()
            .unwrap_or_default();

        self.body_mut().replace(edit_value(from_utf8(body)?).await?);
        Ok(self)
    }
}

impl_opt!(RequestBuilder, crate::Error);
// impl_when!(Request);
// impl_when!(KResult<Request>);
