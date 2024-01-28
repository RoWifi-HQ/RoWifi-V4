use serde::{Deserialize, Serialize};
use time::{serde::rfc3339::option, OffsetDateTime};

use super::id::UserId;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartialUser {
    #[serde(rename = "createTime", default, with = "option")]
    pub create_time: Option<OffsetDateTime>,
    pub id: UserId,
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
}
