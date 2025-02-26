#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions, clippy::missing_panics_doc)]

pub mod error;
pub mod filter;
pub mod request;
mod route;

use filter::AssetFilterBuilder;
use http_body_util::{BodyExt, Full};
use hyper::{
    body::Bytes,
    header::{HeaderName, HeaderValue, AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE},
    http::response::Parts,
    Method, Request as HyperRequest, StatusCode,
};
use hyper_rustls::HttpsConnector;
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client as HyperClient},
    rt::TokioExecutor,
};
use rowifi_roblox_models::{
    datastore::{Datastore, DatastoreEntry, PartialDatastoreEntry},
    group::{Group, GroupRole, GroupUserRole},
    id::{GroupId, UniverseId, UserId},
    inventory::InventoryItem,
    universe::Universe,
    user::{OAuthUser, PartialUser},
};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

use error::DeserializeBodyError;
use request::Request;
use serde_json::Value;

use crate::{
    error::{ErrorKind, RobloxError},
    route::Route,
};

#[derive(Clone)]
pub struct RobloxClient {
    client: HyperClient<HttpsConnector<HttpConnector>, Full<Bytes>>,
    open_cloud_auth: String,
    proxy_url: Option<String>,
}

/// Represents a long-running operation
///
/// See [`Operation`](https://create.roblox.com/docs/reference/cloud/assets/v1#Operation) for details.
#[derive(Deserialize)]
pub struct Operation<T> {
    pub response: T,
}

#[derive(Serialize, Deserialize)]
pub struct VecWrapper<T> {
    pub data: Vec<T>,
}

#[derive(Debug, Deserialize)]
pub struct InventoryItems {
    #[serde(rename = "inventoryItems")]
    pub inventory_items: Vec<InventoryItem>,
    #[serde(rename = "nextPageToken", default)]
    pub next_page_token: Option<String>,
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

#[derive(Debug, Deserialize)]
pub struct DatastoresResponse {
    #[serde(rename = "dataStores")]
    pub datastores: Vec<Datastore>,
    #[serde(rename = "nextPageToken", default)]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DatastoreEntriesResponse {
    #[serde(rename = "dataStoreEntries")]
    pub datastore_entries: Vec<PartialDatastoreEntry>,
    #[serde(rename = "nextPageToken", default)]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub next_page_token: Option<String>,
}

#[derive(Debug)]
pub struct UpdateDatastoreEntryArgs {
    pub value: Value,
    pub users: Vec<UserId>,
    pub attributes: Option<Value>,
}

impl RobloxClient {
    #[must_use]
    pub fn new(open_cloud_auth: &str, proxy_url: Option<String>) -> Self {
        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_or_http()
            .enable_http1()
            .build();
        let client = HyperClient::builder(TokioExecutor::new()).build(connector);
        Self {
            client,
            open_cloud_auth: open_cloud_auth.to_string(),
            proxy_url,
        }
    }

    #[must_use]
    pub fn proxy_uri(&self) -> Option<&str> {
        self.proxy_url.as_deref()
    }

    /// Get the ranks of the user of all the groups they are part of.
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn get_user_roles(&self, user_id: UserId) -> Result<Vec<GroupUserRole>, RobloxError> {
        let route = Route::GetUserGroupRoles { user_id: user_id.0 };

        let request = Request::new()
            .uri(route.to_string())
            .method(Method::GET)
            .proxy_uri(self.proxy_url.clone())
            .body(Full::default())
            .build()
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

        let request = Request::new()
            .uri(route.to_string())
            .method(Method::GET)
            .header(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_str(&self.open_cloud_auth).unwrap(),
            )
            .proxy_uri(self.proxy_url.clone())
            .body(Full::default())
            .build()
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

    /// Get multiple users.
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn get_users(
        &self,
        user_ids: impl Iterator<Item = UserId>,
    ) -> Result<Vec<PartialUser>, RobloxError> {
        let route = Route::GetUsers;

        let user_ids = user_ids.collect::<Vec<_>>();
        let json = serde_json::json!({"userIds": user_ids});
        let body = serde_json::to_vec(&json).map_err(|source| RobloxError {
            source: Some(Box::new(source)),
            kind: ErrorKind::BuildingRequest,
        })?;

        let request = Request::new()
            .uri(route.to_string())
            .method(Method::POST)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
            .header(CONTENT_LENGTH, body.len())
            .proxy_uri(self.proxy_url.clone())
            .body(Full::new(Bytes::from(body)))
            .build()
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
        Ok(json.data)
    }

    /// Get a user from their username.
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn get_users_from_usernames(
        &self,
        usernames: impl Iterator<Item = &str>,
    ) -> Result<Vec<PartialUser>, RobloxError> {
        let route = Route::GetUserByUsernames;

        let usernames = usernames.collect::<Vec<_>>();
        let json = serde_json::json!({"usernames": usernames});
        let body = serde_json::to_vec(&json).map_err(|source| RobloxError {
            source: Some(Box::new(source)),
            kind: ErrorKind::BuildingRequest,
        })?;

        let request = Request::new()
            .uri(route.to_string())
            .method(Method::POST)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
            .header(CONTENT_LENGTH, body.len())
            .proxy_uri(self.proxy_url.clone())
            .body(Full::new(Bytes::from(body)))
            .build()
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
        Ok(json.data)
    }

    /// Get the items in an user's inventory.
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn get_inventory_items(
        &self,
        user_id: UserId,
        asset_filter: AssetFilterBuilder,
    ) -> Result<Vec<InventoryItem>, RobloxError> {
        // We request inventory items indivdually specifically, so if the filter is empty, it means
        // we do not want anything.
        if asset_filter.is_empty() {
            return Ok(Vec::new());
        }

        let route = Route::ListInventoryItems {
            user_id: user_id.0,
            filter: &asset_filter.build(),
        };

        let request = Request::new()
            .uri(route.to_string())
            .method(Method::GET)
            .header(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_str(&self.open_cloud_auth).unwrap(),
            )
            .proxy_uri(self.proxy_url.clone())
            .body(Full::default())
            .build()
            .map_err(|source| RobloxError {
                source: Some(Box::new(source)),
                kind: ErrorKind::BuildingRequest,
            })?;

        let (parts, bytes) = self.request(request).await?;

        if !parts.status.is_success() {
            if parts.status == StatusCode::FORBIDDEN {
                return Ok(Vec::new());
            }
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
                let _ = write!(route, "&pageToken={next_page_token}");
            }
            let request = Request::new()
                .uri(&route)
                .method(Method::GET)
                .header(
                    HeaderName::from_static("x-api-key"),
                    HeaderValue::from_str(&self.open_cloud_auth).unwrap(),
                )
                .body(Full::default())
                .proxy_uri(self.proxy_url.clone())
                .build()
                .map_err(|source| RobloxError {
                    source: Some(Box::new(source)),
                    kind: ErrorKind::BuildingRequest,
                })?;

            let (parts, bytes) = self.request(request).await?;

            if parts.status == StatusCode::BAD_REQUEST {
                return Ok(None);
            }

            if !parts.status.is_success() {
                if parts.status == StatusCode::NOT_FOUND {
                    return Ok(None);
                }

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

        let request = Request::new()
            .uri(route.to_string())
            .method(Method::GET)
            .header(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_str(&self.open_cloud_auth).unwrap(),
            )
            .proxy_uri(self.proxy_url.clone())
            .body(Full::default())
            .build()
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

        let request = Request::new()
            .uri(route.to_string())
            .method(Method::GET)
            .header(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_str(&self.open_cloud_auth).unwrap(),
            )
            .proxy_uri(self.proxy_url.clone())
            .body(Full::default())
            .build()
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
        let request = Request::new()
            .uri(route.to_string())
            .method(Method::GET)
            .header(AUTHORIZATION, HeaderValue::from_str(authorization).unwrap())
            .proxy_uri(self.proxy_url.clone())
            .body(Full::default())
            .build()
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

    /// Get the universe object.
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn get_universe(&self, universe_id: UniverseId) -> Result<Universe, RobloxError> {
        let route = Route::GetUniverse {
            universe_id: universe_id.0,
        };

        let request = Request::new()
            .uri(route.to_string())
            .method(Method::GET)
            .header(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_str(&self.open_cloud_auth).unwrap(),
            )
            .proxy_uri(self.proxy_url.clone())
            .body(Full::default())
            .build()
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

        let json = serde_json::from_slice(&bytes).map_err(|source| RobloxError {
            source: Some(Box::new(DeserializeBodyError {
                source: Some(Box::new(source)),
                bytes,
            })),
            kind: ErrorKind::Deserialize,
        })?;

        Ok(json)
    }

    /// Get a list of datastores of an universe.
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn list_datastores(
        &self,
        universe_id: UniverseId,
    ) -> Result<PaginatedResponse<Datastore>, RobloxError> {
        let route = Route::ListDatastores {
            universe_id: universe_id.0,
            query: "",
        };

        let request = Request::new()
            .uri(route.to_string())
            .method(Method::GET)
            .header(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_str(&self.open_cloud_auth).unwrap(),
            )
            .proxy_uri(self.proxy_url.clone())
            .body(Full::default())
            .build()
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
            serde_json::from_slice::<DatastoresResponse>(&bytes).map_err(|source| RobloxError {
                source: Some(Box::new(DeserializeBodyError {
                    source: Some(Box::new(source)),
                    bytes,
                })),
                kind: ErrorKind::Deserialize,
            })?;

        Ok(PaginatedResponse {
            data: json.datastores,
            next_page_token: json.next_page_token,
        })
    }

    /// Lists the entries of a datastore. Supports filtering based on entry IDs.
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn list_datastore_entries(
        &self,
        universe_id: UniverseId,
        datastore_id: &str,
        page_token: &str,
        page_size: u32,
        filter: Option<&str>,
    ) -> Result<PaginatedResponse<PartialDatastoreEntry>, RobloxError> {
        let route = Route::ListDatastoreEntries {
            universe_id: universe_id.0,
            datastore_id,
            page_token,
            page_size,
            filter,
        };

        let request = Request::new()
            .uri(route.to_string())
            .method(Method::GET)
            .header(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_str(&self.open_cloud_auth).unwrap(),
            )
            .proxy_uri(self.proxy_url.clone())
            .body(Full::default())
            .build()
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
            serde_json::from_slice::<DatastoreEntriesResponse>(&bytes).map_err(|source| {
                RobloxError {
                    source: Some(Box::new(DeserializeBodyError {
                        source: Some(Box::new(source)),
                        bytes,
                    })),
                    kind: ErrorKind::Deserialize,
                }
            })?;

        Ok(PaginatedResponse {
            data: json.datastore_entries,
            next_page_token: json.next_page_token,
        })
    }

    /// Get a datastore entry.
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn get_datastore_entry(
        &self,
        universe_id: UniverseId,
        datastore_id: &str,
        entry_id: &str,
        revision_id: Option<&str>,
    ) -> Result<DatastoreEntry, RobloxError> {
        let route = Route::GetDatastoreEntry {
            universe_id: universe_id.0,
            datastore_id,
            entry_id,
            revision_id,
        };

        let request = Request::new()
            .uri(route.to_string())
            .method(Method::GET)
            .header(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_str(&self.open_cloud_auth).unwrap(),
            )
            .proxy_uri(self.proxy_url.clone())
            .body(Full::default())
            .build()
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

        let json = serde_json::from_slice(&bytes).map_err(|source| RobloxError {
            source: Some(Box::new(DeserializeBodyError {
                source: Some(Box::new(source)),
                bytes,
            })),
            kind: ErrorKind::Deserialize,
        })?;

        Ok(json)
    }

    /// Update a datastore entry.
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn update_datastore_entry(
        &self,
        universe_id: UniverseId,
        datastore_id: &str,
        entry_id: &str,
        args: UpdateDatastoreEntryArgs,
    ) -> Result<DatastoreEntry, RobloxError> {
        let route = Route::UpdateDatastoreEntry {
            universe_id: universe_id.0,
            datastore_id,
            entry_id,
        };

        let json = serde_json::json!({"value": args.value, "users": args.users, "attributes": args.attributes});
        let body = serde_json::to_vec(&json).map_err(|source| RobloxError {
            source: Some(Box::new(source)),
            kind: ErrorKind::BuildingRequest,
        })?;

        let request = Request::new()
            .uri(route.to_string())
            .method(Method::PATCH)
            .header(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_str(&self.open_cloud_auth).unwrap(),
            )
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
            .header(CONTENT_LENGTH, body.len())
            .proxy_uri(self.proxy_url.clone())
            .body(Full::new(Bytes::from(body)))
            .build()
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

        let json = serde_json::from_slice(&bytes).map_err(|source| RobloxError {
            source: Some(Box::new(DeserializeBodyError {
                source: Some(Box::new(source)),
                bytes,
            })),
            kind: ErrorKind::Deserialize,
        })?;

        Ok(json)
    }

    /// Delete a datastore entry.
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn delete_datastore_entry(
        &self,
        universe_id: UniverseId,
        datastore_id: &str,
        entry_id: &str,
    ) -> Result<(), RobloxError> {
        let route = Route::DeleteDatastoreEntry {
            universe_id: universe_id.0,
            datastore_id,
            entry_id,
        };

        let request = Request::new()
            .uri(route.to_string())
            .method(Method::DELETE)
            .header(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_str(&self.open_cloud_auth).unwrap(),
            )
            .proxy_uri(self.proxy_url.clone())
            .body(Full::default())
            .build()
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

        Ok(())
    }

    /// Creates a datastore entry.
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn create_datastore_entry(
        &self,
        universe_id: UniverseId,
        datastore_id: &str,
        entry_id: &str,
        args: UpdateDatastoreEntryArgs,
    ) -> Result<DatastoreEntry, RobloxError> {
        let route = Route::CreateDatastoreEntry {
            universe_id: universe_id.0,
            datastore_id,
            entry_id,
        };

        let json = serde_json::json!({"value": args.value, "users": args.users, "attributes": args.attributes});
        let body = serde_json::to_vec(&json).map_err(|source| RobloxError {
            source: Some(Box::new(source)),
            kind: ErrorKind::BuildingRequest,
        })?;

        let request = Request::new()
            .uri(route.to_string())
            .method(Method::POST)
            .header(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_str(&self.open_cloud_auth).unwrap(),
            )
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
            .header(CONTENT_LENGTH, body.len())
            .proxy_uri(self.proxy_url.clone())
            .body(Full::new(Bytes::from(body)))
            .build()
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

        let json = serde_json::from_slice(&bytes).map_err(|source| RobloxError {
            source: Some(Box::new(DeserializeBodyError {
                source: Some(Box::new(source)),
                bytes,
            })),
            kind: ErrorKind::Deserialize,
        })?;

        Ok(json)
    }

    /// Lists the revisions of an entry.
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn list_datastore_entry_revisions(
        &self,
        universe_id: UniverseId,
        datastore_id: &str,
        entry_id: &str,
        page_token: &str,
        page_size: u32,
    ) -> Result<PaginatedResponse<PartialDatastoreEntry>, RobloxError> {
        let route = Route::ListDatastoreEntryRevisions {
            universe_id: universe_id.0,
            datastore_id,
            entry_id,
            page_token,
            page_size,
        };

        let request = Request::new()
            .uri(route.to_string())
            .method(Method::GET)
            .header(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_str(&self.open_cloud_auth).unwrap(),
            )
            .proxy_uri(self.proxy_url.clone())
            .body(Full::default())
            .build()
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
            serde_json::from_slice::<DatastoreEntriesResponse>(&bytes).map_err(|source| {
                RobloxError {
                    source: Some(Box::new(DeserializeBodyError {
                        source: Some(Box::new(source)),
                        bytes,
                    })),
                    kind: ErrorKind::Deserialize,
                }
            })?;

        Ok(PaginatedResponse {
            data: json.datastore_entries,
            next_page_token: json.next_page_token,
        })
    }

    /// Make a request to the Roblox API.
    ///
    /// # Errors
    ///
    /// See [`RobloxError`] for details.
    pub async fn request(
        &self,
        request: HyperRequest<Full<Bytes>>,
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
