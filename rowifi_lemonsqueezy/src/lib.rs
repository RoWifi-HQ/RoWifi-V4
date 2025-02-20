#![deny(clippy::all, clippy::pedantic)]
#![allow(
    clippy::similar_names,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]

mod error;
pub mod model;

use http_body_util::{BodyExt, Full};
use hyper::{
    body::Bytes,
    header::{ACCEPT, AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE},
    http::HeaderValue,
    Method, Request, StatusCode,
};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client as HyperClient},
    rt::TokioExecutor,
};

use model::{LicenseActivation, LicenseDeactivation, LicenseValidation};

pub use error::Error;

#[derive(Clone)]
pub struct Client {
    client: HyperClient<HttpsConnector<HttpConnector>, Full<Bytes>>,
    lemon_key: String,
}

impl Client {
    #[must_use]
    pub fn new(lemon_key: &str) -> Self {
        let connector = HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_or_http()
            .enable_http1()
            .build();
        let client = HyperClient::builder(TokioExecutor::new()).build(connector);
        Self {
            client,
            lemon_key: lemon_key.to_string(),
        }
    }

    pub async fn request(
        &self,
        url: &str,
        method: Method,
        body: Option<Vec<u8>>,
    ) -> Result<Response, Error> {
        let builder = Request::builder()
            .uri(url)
            .method(method)
            .header(AUTHORIZATION, format!("Bearer: {}", &self.lemon_key));
        let req = if let Some(bytes) = body {
            let len = bytes.len();
            builder
                .header(
                    CONTENT_TYPE,
                    HeaderValue::from_static("application/x-www-form-urlencoded"),
                )
                .header(ACCEPT, HeaderValue::from_static("application/json"))
                .header(CONTENT_LENGTH, len)
                .body(Full::from(Bytes::from(bytes)))?
        } else {
            builder.body(Full::new(Bytes::new()))?
        };

        let res = self.client.request(req).await?;
        let status = res.status();

        let bytes = res.into_body().collect().await?.to_bytes().to_vec();

        Ok(Response { status, bytes })
    }

    pub async fn activate_license(
        &self,
        license_key: &str,
        instance_name: &str,
    ) -> Result<LicenseActivation, Error> {
        let route = "https://api.lemonsqueezy.com/v1/licenses/activate";
        let data = &[
            ("license_key", license_key),
            ("instance_name", instance_name),
        ];
        let body = serde_urlencoded::to_string(data).unwrap();
        let res = self
            .request(route, Method::POST, Some(body.into_bytes()))
            .await?;
        if !res.status.is_success() {
            return Err(Error::APIError(res.status));
        }
        Ok(serde_json::from_slice(&res.bytes)?)
    }

    pub async fn deactivate_license(
        &self,
        license_key: &str,
        instance_id: &str,
    ) -> Result<LicenseDeactivation, Error> {
        let route = "https://api.lemonsqueezy.com/v1/licenses/deactivate";
        let data = &[("license_key", license_key), ("instance_id", instance_id)];
        let body = serde_urlencoded::to_string(data).unwrap();
        let res = self
            .request(route, Method::POST, Some(body.into_bytes()))
            .await?;
        Ok(serde_json::from_slice(&res.bytes)?)
    }

    pub async fn validate_license(
        &self,
        license_key: &str,
        instance_id: &str,
    ) -> Result<LicenseValidation, Error> {
        let route = "https://api.lemonsqueezy.com/v1/licenses/validate";
        let data = &[("license_key", license_key), ("instance_id", instance_id)];
        let body = serde_urlencoded::to_string(data).unwrap();
        let res = self
            .request(route, Method::POST, Some(body.into_bytes()))
            .await?;
        Ok(serde_json::from_slice(&res.bytes)?)
    }
}

pub struct Response {
    pub status: StatusCode,
    bytes: Vec<u8>,
}
