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
    /// The timestamp when the data store entry was created.
    #[serde(rename = "createTime", default)]
    pub create_time: Option<DateTime<Utc>>,
    /// The revision ID of the data store entry.
    ///
    /// A new revision is committed whenever the data store entry is changed in any way.
    ///
    /// The format is an arbitrary string. Example: "foo".
    #[serde(rename = "revisionId", default)]
    pub revision_id: Option<String>,
    /// The timestamp when the revision was created.
    #[serde(rename = "revisionCreateTime", default)]
    pub revision_create_time: Option<DateTime<Utc>>,
    /// The state of the data store entry.
    #[serde(default)]
    pub state: Option<DatastoreEntryState>,
    /// This checksum is computed by the server based on the value of other fields, and may be sent on update and delete requests (and potentially on certain custom methods) to ensure the client has an up-to-date value before proceeding.
    #[serde(default)]
    pub etag: Option<String>,
    /// The value of the entry.
    #[serde(default)]
    pub value: Option<Value>,
    /// The resource ID of the entry.
    pub id: String,
    /// Users associated with the entry.
    #[serde(default)]
    pub users: Option<Vec<UserId>>,
    /// An arbitrary set of attributes associated with the entry.
    #[serde(default)]
    pub attributes: Option<Value>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_datastore_entry_state_serialization() {
        let active_state = DatastoreEntryState::Active;
        let serialized = serde_json::to_string(&active_state).expect("Serialization failed");
        assert_eq!(serialized, "\"ACTIVE\"");

        let deleted_state = DatastoreEntryState::Deleted;
        let serialized = serde_json::to_string(&deleted_state).expect("Serialization failed");
        assert_eq!(serialized, "\"DELETED\"");

        let unspecified_state = DatastoreEntryState::Unspecified;
        let serialized = serde_json::to_string(&unspecified_state).expect("Serialization failed");
        assert_eq!(serialized, "\"STATE_UNSPECIFIED\"");
    }

    #[test]
    fn test_datastore_entry_state_deserialization() {
        let json = "\"ACTIVE\"";
        let deserialized: DatastoreEntryState = serde_json::from_str(json).expect("Deserialization failed");
        assert_eq!(deserialized, DatastoreEntryState::Active);

        let json = "\"DELETED\"";
        let deserialized: DatastoreEntryState = serde_json::from_str(json).expect("Deserialization failed");
        assert_eq!(deserialized, DatastoreEntryState::Deleted);

        let json = "\"STATE_UNSPECIFIED\"";
        let deserialized: DatastoreEntryState = serde_json::from_str(json).expect("Deserialization failed");
        assert_eq!(deserialized, DatastoreEntryState::Unspecified);
    }
}
