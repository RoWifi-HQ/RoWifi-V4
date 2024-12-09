use serde::{Deserialize, Serialize};

use super::id::{GroupId, RoleId};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartialGroup {
    pub id: GroupId,
    pub name: String,
    #[serde(rename = "memberCount")]
    pub member_count: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartialRank {
    pub id: RoleId,
    #[serde(default)]
    pub name: Option<String>,
    pub rank: u32,
    #[serde(rename = "memberCount")]
    pub member_count: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Group {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GroupUserRole {
    pub group: PartialGroup,
    pub role: PartialRank,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GroupRole {
    pub id: String,
    pub rank: u32,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "memberCount")]
    pub member_count: Option<i64>,
}
