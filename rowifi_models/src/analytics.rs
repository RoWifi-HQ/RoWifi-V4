use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_postgres::{types::Json, Error, Row};

use crate::roblox::id::GroupId;

#[derive(Debug, Deserialize, Serialize)]
pub struct AnalyticsGroup {
    pub group_id: GroupId,
    pub roles: Vec<AnalyticsRole>,
    pub member_count: i64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AnalyticsRole {
    pub id: i64,
    pub rank: u32,
    pub member_count: i64,
}

impl TryFrom<Row> for AnalyticsGroup {
    type Error = Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        let group_id = row.try_get("group_id")?;
        let roles = row.try_get("roles").unwrap_or_else(|_| Json(Vec::new()));
        let member_count = row.try_get("member_count")?;
        let timestamp = row.try_get("timestamp")?;

        Ok(Self {
            group_id,
            roles: roles.0,
            member_count,
            timestamp,
        })
    }
}
