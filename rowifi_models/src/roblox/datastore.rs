use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Datastore {
    pub id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartialDatastoreEntry {
    pub id: String
}