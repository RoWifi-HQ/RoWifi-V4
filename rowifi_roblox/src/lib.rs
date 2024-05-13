#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod error;
pub mod filter;
mod route;

use error::DeserializeBodyError;
use http_body_util::{BodyExt, Full};
use hyper::{
    body::Bytes,
    header::{HeaderValue, AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE},
    http::response::Parts,
    Method, Request, StatusCode,
};
use hyper_rustls::HttpsConnector;
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client as HyperClient},
    rt::TokioExecutor,
};
use rowifi_models::roblox::{
    group::{Group, GroupRole, GroupUserRole},
    id::{GroupId, UserId},
    inventory::InventoryItem,
    user::{OAuthUser, PartialUser},
    Operation,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::{ErrorKind, RobloxError},
    route::Route,
};

#[derive(Clone)]
pub struct RobloxClient {
    client: HyperClient<HttpsConnector<HttpConnector>, Full<Bytes>>,
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
    #[serde(rename = "nextPageToken", default)]
    pub next_page_token: Option<String>,
}

#[derive(Deserialize)]
pub struct ThumbnailResponse {
    #[serde(rename = "imageUri")]
    pub image_uri: String,
}

impl RobloxClient {
    #[must_use]
    pub fn new(open_cloud_auth: &str) -> Self {
        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_or_http()
            .enable_http1()
            .build();
        let client = HyperClient::builder(TokioExecutor::new()).build(connector);
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
            .body(Full::default())
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
            .body(Full::default())
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

    /// Get a user from their username.
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn get_user_from_username(
        &self,
        username: &str,
    ) -> Result<Option<PartialUser>, RobloxError> {
        let route = Route::GetUserByUsernames;

        let usernames = vec![username];
        let json = serde_json::json!({"usernames": usernames});
        let body = serde_json::to_vec(&json).map_err(|source| RobloxError {
            source: Some(Box::new(source)),
            kind: ErrorKind::BuildingRequest,
        })?;

        let request = Request::builder()
            .uri(route.to_string())
            .method(Method::POST)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
            .header(CONTENT_LENGTH, body.len())
            .body(Full::new(Bytes::from(body)))
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

        let json = serde_json::from_slice::<VecWrapper<PartialUser>>(&bytes).map_err(|source| {
            RobloxError {
                source: Some(Box::new(DeserializeBodyError {
                    source: Some(Box::new(source)),
                    bytes,
                })),
                kind: ErrorKind::Deserialize,
            }
        })?;
        Ok(json.data.into_iter().next())
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
            .body(Full::default())
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

    /// Get the ranks of a Roblox Group. Follows the pagination if necessary.
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn get_group_ranks(
        &self,
        group_id: GroupId,
    ) -> Result<Option<Vec<GroupRole>>, RobloxError> {
        let route = Route::ListGroupRanks {
            group_id: group_id.0,
        };
        let mut ranks = Vec::new();
        let mut next_page_token = None;

        loop {
            let mut route = route.to_string();
            if let Some(next_page_token) = next_page_token {
                route.push_str(&format!("&pageToken={next_page_token}"));
            }
            let request = Request::builder()
                .uri(&route)
                .method(Method::GET)
                .header("x-api-key", &self.open_cloud_auth)
                .body(Full::default())
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
                        route,
                        status: parts.status,
                        bytes,
                    },
                });
            }

            let json =
                serde_json::from_slice::<GroupRanks>(&bytes).map_err(|source| RobloxError {
                    source: Some(Box::new(DeserializeBodyError {
                        source: Some(Box::new(source)),
                        bytes,
                    })),
                    kind: ErrorKind::Deserialize,
                })?;
            tracing::trace!(?json);
            ranks.extend(json.ranks.into_iter());
            if let Some(npt) = json.next_page_token {
                if npt.is_empty() {
                    break;
                }
                next_page_token = Some(npt);
            } else {
                break;
            }
        }

        Ok(Some(ranks))
    }

    /// Get a Roblox Group
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn get_group(&self, group_id: GroupId) -> Result<Option<Group>, RobloxError> {
        let route = Route::GetGroup {
            group_id: group_id.0,
        };

        let request = Request::builder()
            .uri(route.to_string())
            .method(Method::GET)
            .header("x-api-key", &self.open_cloud_auth)
            .body(Full::default())
            .map_err(|source| RobloxError {
                source: Some(Box::new(source)),
                kind: ErrorKind::BuildingRequest,
            })?;

        let (parts, bytes) = self.request(request).await?;

        if parts.status == StatusCode::NOT_FOUND {
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

        let json = serde_json::from_slice::<Group>(&bytes).map_err(|source| RobloxError {
            source: Some(Box::new(DeserializeBodyError {
                source: Some(Box::new(source)),
                bytes,
            })),
            kind: ErrorKind::Deserialize,
        })?;

        Ok(Some(json))
    }

    /// Get a user's thumbnail.
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn get_user_thumbnail(&self, user_id: UserId) -> Result<String, RobloxError> {
        let route = Route::GetUserThumbail { user_id: user_id.0 };

        let request = Request::builder()
            .uri(route.to_string())
            .method(Method::GET)
            .header("x-api-key", &self.open_cloud_auth)
            .body(Full::default())
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
            serde_json::from_slice::<Operation<ThumbnailResponse>>(&bytes).map_err(|source| {
                RobloxError {
                    source: Some(Box::new(DeserializeBodyError {
                        source: Some(Box::new(source)),
                        bytes,
                    })),
                    kind: ErrorKind::Deserialize,
                }
            })?;

        Ok(json.response.image_uri)
    }

    /// Get an user from the Open Cloud API using the OAuth workflow.
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn get_oauth_userinfo(&self, authorization: &str) -> Result<OAuthUser, RobloxError> {
        let route = Route::OAuthUserInfo;
        let request = Request::builder()
            .uri(route.to_string())
            .method(Method::GET)
            .header(AUTHORIZATION, authorization)
            .body(Full::default())
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

        let json = serde_json::from_slice::<OAuthUser>(&bytes).map_err(|source| RobloxError {
            source: Some(Box::new(DeserializeBodyError {
                source: Some(Box::new(source)),
                bytes,
            })),
            kind: ErrorKind::Deserialize,
        })?;

        Ok(json)
    }

    /// Make a request to the Roblox API.
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn request(
        &self,
        request: Request<Full<Bytes>>,
    ) -> Result<(Parts, Vec<u8>), RobloxError> {
        let res = self
            .client
            .request(request)
            .await
            .map_err(|source| RobloxError {
                source: Some(Box::new(source)),
                kind: ErrorKind::Sending,
            })?;

        let (parts, body) = res.into_parts();
        let bytes = body
            .collect()
            .await
            .map_err(|source| RobloxError {
                source: Some(Box::new(source)),
                kind: ErrorKind::ChunkingResponse,
            })?
            .to_bytes();

        Ok((parts, bytes.into()))
    }
}
