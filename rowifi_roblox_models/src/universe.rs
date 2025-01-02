use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Universe {
    pub path: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub description: String,
    pub user: Option<String>,
    pub group: Option<String>,
}
