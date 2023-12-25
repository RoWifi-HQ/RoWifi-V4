pub mod error;
mod route;

use error::DeserializeBodyError;
use hyper::{
    body::{self, Buf},
    client::HttpConnector,
    Body, Client as HyperClient, Method, Request,
};
use hyper_rustls::HttpsConnector;
use rowifi_models::roblox::{group::GroupUserRole, id::UserId};
use serde::{Deserialize, Serialize};

use crate::{
    error::{ErrorKind, RobloxError},
    route::Route,
};

#[derive(Clone)]
pub struct RobloxClient {
    client: HyperClient<HttpsConnector<HttpConnector>>,
}

#[derive(Serialize, Deserialize)]
pub struct VecWrapper<T> {
    pub data: Vec<T>,
}

impl RobloxClient {
    pub fn new() -> Self {
        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .build();
        let client = HyperClient::builder().build(connector);
        Self { client }
    }

    pub async fn get_user_roles(&self, user_id: UserId) -> Result<Vec<GroupUserRole>, RobloxError> {
        let route = Route::UserGroupRoles { user_id: user_id.0 };

        let request = Request::builder()
            .uri(format!("{route}"))
            .method(Method::GET)
            .body(Body::empty())
            .map_err(|source| RobloxError {
                source: Some(Box::new(source)),
                kind: ErrorKind::BuildingRequest,
            })?;

        let res = self
            .client
            .request(request)
            .await
            .map_err(|source| RobloxError {
                source: Some(Box::new(source)),
                kind: ErrorKind::Sending,
            })?;

        let (parts, body) = res.into_parts();
        let mut buf = body::aggregate(body).await.map_err(|source| RobloxError {
            source: Some(Box::new(source)),
            kind: ErrorKind::ChunkingResponse,
        })?;
        let mut bytes = vec![0; buf.remaining()];
        buf.copy_to_slice(&mut bytes);

        let json =
            serde_json::from_slice::<VecWrapper<GroupUserRole>>(&bytes).map_err(|source| {
                RobloxError {
                    source: Some(Box::new(DeserializeBodyError {
                        source: Some(Box::new(source)),
                        bytes,
                    })),
                    kind: ErrorKind::Response {
                        route: route.to_string(),
                        status: parts.status,
                    },
                }
            })?;

        Ok(json.data)
    }
}
