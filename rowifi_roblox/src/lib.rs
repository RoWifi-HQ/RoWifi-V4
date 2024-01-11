#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod error;
pub mod filter;
mod route;

use error::DeserializeBodyError;
use hyper::{
    body::{self, Buf},
    client::HttpConnector,
    http::response::Parts,
    Body, Client as HyperClient, Method, Request, StatusCode,
};
use hyper_rustls::HttpsConnector;
use rowifi_models::roblox::{
    group::{GroupRole, GroupUserRole},
    id::{GroupId, UserId},
    inventory::InventoryItem,
    user::PartialUser,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::{ErrorKind, RobloxError},
    route::Route,
};

#[derive(Clone)]
pub struct RobloxClient {
    client: HyperClient<HttpsConnector<HttpConnector>>,
    open_cloud_auth: String,
}

#[derive(Serialize, Deserialize)]
pub struct VecWrapper<T> {
    pub data: Vec<T>,
}

#[derive(Debug, Deserialize)]
pub struct InventoryItems {
    #[serde(rename = "inventoryItems")]
    pub inventory_items: Vec<InventoryItem>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: String,
}

#[derive(Debug, Deserialize)]
pub struct GroupRanks {
    #[serde(rename = "groupRoles")]
    pub ranks: Vec<GroupRole>,
}

impl RobloxClient {
    #[must_use]
    pub fn new(open_cloud_auth: &str) -> Self {
        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .build();
        let client = HyperClient::builder().build(connector);
        Self {
            client,
            open_cloud_auth: open_cloud_auth.to_string(),
        }
    }

    /// Get the ranks of the user of all the groups they are part of.
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn get_user_roles(&self, user_id: UserId) -> Result<Vec<GroupUserRole>, RobloxError> {
        let route = Route::GetUserGroupRoles { user_id: user_id.0 };

        let request = Request::builder()
            .uri(route.to_string())
            .method(Method::GET)
            .body(Body::empty())
            .map_err(|source| RobloxError {
                source: Some(Box::new(source)),
                kind: ErrorKind::BuildingRequest,
            })?;

        let (parts, bytes) = self.request(request).await?;

        if !parts.status.is_success() {
            return Err(RobloxError {
                source: None,
                kind: ErrorKind::Response {
                    route: route.to_string(),
                    status: parts.status,
                    bytes,
                },
            });
        }

        let json =
            serde_json::from_slice::<VecWrapper<GroupUserRole>>(&bytes).map_err(|source| {
                RobloxError {
                    source: Some(Box::new(DeserializeBodyError {
                        source: Some(Box::new(source)),
                        bytes,
                    })),
                    kind: ErrorKind::Deserialize,
                }
            })?;

        Ok(json.data)
    }

    /// Gets the user from the Roblox Open Cloud API.
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn get_user(&self, user_id: UserId) -> Result<PartialUser, RobloxError> {
        let route = Route::GetUser { user_id: user_id.0 };

        let request = Request::builder()
            .uri(route.to_string())
            .method(Method::GET)
            .header("x-api-key", &self.open_cloud_auth)
            .body(Body::empty())
            .map_err(|source| RobloxError {
                source: Some(Box::new(source)),
                kind: ErrorKind::BuildingRequest,
            })?;

        let (parts, bytes) = self.request(request).await?;

        if !parts.status.is_success() {
            return Err(RobloxError {
                source: None,
                kind: ErrorKind::Response {
                    route: route.to_string(),
                    status: parts.status,
                    bytes,
                },
            });
        }

        let json = serde_json::from_slice::<PartialUser>(&bytes).map_err(|source| RobloxError {
            source: Some(Box::new(DeserializeBodyError {
                source: Some(Box::new(source)),
                bytes,
            })),
            kind: ErrorKind::Deserialize,
        })?;

        Ok(json)
    }

    /// Get the items in an user's inventory.
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn get_inventory_items(
        &self,
        user_id: UserId,
        asset_filter: String,
    ) -> Result<Vec<InventoryItem>, RobloxError> {
        let route = Route::ListInventoryItems {
            user_id: user_id.0,
            filter: asset_filter,
        };

        let request = Request::builder()
            .uri(route.to_string())
            .method(Method::GET)
            .header("x-api-key", &self.open_cloud_auth)
            .body(Body::empty())
            .map_err(|source| RobloxError {
                source: Some(Box::new(source)),
                kind: ErrorKind::BuildingRequest,
            })?;

        let (parts, bytes) = self.request(request).await?;

        if !parts.status.is_success() {
            return Err(RobloxError {
                source: None,
                kind: ErrorKind::Response {
                    route: route.to_string(),
                    status: parts.status,
                    bytes,
                },
            });
        }

        let json =
            serde_json::from_slice::<InventoryItems>(&bytes).map_err(|source| RobloxError {
                source: Some(Box::new(DeserializeBodyError {
                    source: Some(Box::new(source)),
                    bytes,
                })),
                kind: ErrorKind::Deserialize,
            })?;

        Ok(json.inventory_items)
    }

    pub async fn get_group_ranks(
        &self,
        group_id: GroupId,
    ) -> Result<Option<Vec<GroupRole>>, RobloxError> {
        let route = Route::ListGroupRanks {
            group_id: group_id.0,
        };

        let request = Request::builder()
            .uri(route.to_string())
            .method(Method::GET)
            .header("x-api-key", &self.open_cloud_auth)
            .body(Body::empty())
            .map_err(|source| RobloxError {
                source: Some(Box::new(source)),
                kind: ErrorKind::BuildingRequest,
            })?;

        let (parts, bytes) = self.request(request).await?;

        if parts.status == StatusCode::BAD_REQUEST {
            return Ok(None);
        }

        if !parts.status.is_success() {
            return Err(RobloxError {
                source: None,
                kind: ErrorKind::Response {
                    route: route.to_string(),
                    status: parts.status,
                    bytes,
                },
            });
        }

        let json = serde_json::from_slice::<GroupRanks>(&bytes).map_err(|source| RobloxError {
            source: Some(Box::new(DeserializeBodyError {
                source: Some(Box::new(source)),
                bytes,
            })),
            kind: ErrorKind::Deserialize,
        })?;

        Ok(Some(json.ranks))
    }

    async fn request(&self, request: Request<Body>) -> Result<(Parts, Vec<u8>), RobloxError> {
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

        Ok((parts, bytes))
    }
}
