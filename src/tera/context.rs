use crate::KResult;
use http::Version;
use reqwest::Response;
use serde::ser::Serialize;
use tera::Context;

pub struct ContextBuilder {
    data: Context,
}

impl ContextBuilder {
    pub fn new() -> Self {
        ContextBuilder {
            data: Context::new(),
        }
    }

    pub fn insert<T: Serialize + ?Sized, S: Into<String>>(mut self, key: S, val: &T) -> Self {
        self.data.insert(key.into(), val);
        self
    }

    pub async fn insert_response(mut self, response: Response) -> KResult<Self> {
        self.data.insert("resp_status", response.status().as_str());

        let headers = response.headers();
        for (name, value) in headers.iter() {
            self.data
                .insert(&format!("resp_headers_{}", name), &value.to_str()?);
        }

        let version = response.version();
        match version {
            Version::HTTP_09 => self.data.insert("resp_http_version", &"HTTP/0.9"),
            Version::HTTP_10 => self.data.insert("resp_http_version", &"HTTP/1.0"),
            Version::HTTP_11 => self.data.insert("resp_http_version", &"HTTP/1.1"),
            Version::HTTP_2 => self.data.insert("resp_http_version", &"HTTP/2.0"),
            Version::HTTP_3 => self.data.insert("resp_http_version", &"HTTP/3.0"),
            _ => self.data.insert("resp_http_version", &"Unknown"),
        }

        let content = response.text().await?;
        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(v) => match Context::from_value(v) {
                Ok(v) => self.data.extend(v),
                _ => (),
            },
            _ => (),
        }

        self.data.insert("resp_body", &content);

        Ok(self)
    }

    pub fn build(self) -> Context {
        self.data
    }
}
