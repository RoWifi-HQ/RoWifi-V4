use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::id::UserId;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartialUser {
    #[serde(rename = "createTime", default)]
    pub create_time: Option<DateTime<Utc>>,
    pub id: UserId,
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OAuthUser {
    pub sub: String,
    pub name: String,
}
