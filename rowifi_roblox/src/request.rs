use http_body_util::Full;
use hyper::{
    body::Bytes,
    header::{HeaderName, HeaderValue},
    HeaderMap, Method, Request as HyperRequest,
};
use itertools::Itertools;
use std::collections::HashMap;

#[derive(Clone)]
pub struct Request {
    uri: Option<String>,
    method: Option<Method>,
    headers: HeaderMap,
    body: Option<Full<Bytes>>,

    proxy_uri: Option<String>,
    proxy_params: HashMap<String, String>,
}

impl Default for Request {
    fn default() -> Self {
        Self::new()
    }
}

impl Request {
    #[must_use]
    pub fn new() -> Self {
        Self {
            uri: None,
            method: None,
            headers: HeaderMap::new(),
            body: None,
            proxy_uri: None,
            proxy_params: HashMap::new(),
        }
    }

    #[must_use]
    pub fn uri(mut self, uri: impl Into<String>) -> Self {
        self.uri = Some(uri.into());
        self
    }

    #[must_use]
    pub fn method(mut self, method: Method) -> Self {
        self.method = Some(method);
        self
    }

    #[must_use]
    pub fn header(mut self, name: impl Into<HeaderName>, value: impl Into<HeaderValue>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    #[must_use]
    pub fn body(mut self, body: Full<Bytes>) -> Self {
        self.body = Some(body);
        self
    }

    #[must_use]
    pub fn proxy_uri(mut self, proxy_uri: Option<String>) -> Self {
        self.proxy_uri = proxy_uri;
        self
    }

    #[must_use]
    pub fn proxy_param(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.proxy_params.insert(name.into(), value.into());
        self
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn build(mut self) -> Result<HyperRequest<Full<Bytes>>, hyper::http::Error> {
        let original_uri = self.uri.unwrap_or_default();
        let uri = if let Some(proxy_uri) = &self.proxy_uri {
            self.proxy_params
                .insert("url".into(), urlencoding::encode(&original_uri).to_string());
            proxy_uri.clone()
        } else {
            original_uri
        };

        let final_uri = if self.proxy_uri.is_some() {
            let proxy_query = self
                .proxy_params
                .into_iter()
                .map(|(k, v)| format!("{k}={v}"))
                .join("&");
            format!("{uri}?{proxy_query}")
        } else {
            uri
        };
        tracing::debug!("{}", final_uri);

        let mut builder = HyperRequest::builder().uri(final_uri);
        for header in self.headers {
            builder = builder.header(header.0.unwrap(), header.1);
        }
        if let Some(method) = self.method {
            builder = builder.method(method);
        }
        if let Some(body) = self.body {
            builder.body(body)
        } else {
            builder.body(Full::default())
        }
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }
}
