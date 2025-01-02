use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::id::UserId;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Datastore {
    pub id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartialDatastoreEntry {
    pub id: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum DatastoreEntryState {
    #[serde(rename = "STATE_UNSPECIFIED")]
    Unspecified,
    #[serde(rename = "ACTIVE")]
    Active,
    #[serde(rename = "DELETED")]
    Deleted,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DatastoreEntry {
    /// The timestamp when the data store entry was created.
    #[serde(rename = "createTime")]
    pub create_time: DateTime<Utc>,
    /// The revision ID of the data store entry.
    ///
    /// A new revision is committed whenever the data store entry is changed in any way.
    ///
    /// The format is an arbitrary string. Example: "foo".
    #[serde(rename = "revisionId")]
    pub revision_id: String,
    /// The timestamp when the revision was created.
    #[serde(rename = "revisionCreateTime")]
    pub revision_create_time: DateTime<Utc>,
    /// The state of the data store entry.
    pub state: DatastoreEntryState,
    /// This checksum is computed by the server based on the value of other fields, and may be sent on update and delete requests (and potentially on certain custom methods) to ensure the client has an up-to-date value before proceeding.
    pub etag: String,
    /// The value of the entry.
    pub value: Value,
    /// The resource ID of the entry.
    pub id: String,
    /// Users associated with the entry.
    #[serde(default)]
    pub users: Vec<UserId>,
    /// An arbitrary set of attributes associated with the entry.
    pub attributes: Value,
}
