use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{id::GuildId, roblox::id::UserId as RobloxUserId};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventType {
    pub id: u32,
    pub name: String,
    pub disabled: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct EventLog {
    pub guild_id: GuildId,
    pub event_type: i32,
    pub guild_event_id: i64,
    pub host_id: RobloxUserId,
    pub timestamp: DateTime<Utc>,
    pub attendees: Vec<RobloxUserId>,
    pub notes: Option<String>,
}

impl TryFrom<tokio_postgres::Row> for EventLog {
    type Error = tokio_postgres::Error;

    fn try_from(row: tokio_postgres::Row) -> Result<Self, Self::Error> {
        let guild_id = row.try_get("guild_id")?;
        let event_type = row.try_get("event_type")?;
        let guild_event_id = row.try_get("guild_event_id")?;
        let host_id = row.try_get("host_id")?;
        let timestamp = row.try_get("timestamp")?;
        let attendees = row.try_get("attendees")?;
        let notes = row.try_get("notes")?;

        Ok(Self {
            guild_id,
            event_type,
            guild_event_id,
            host_id,
            timestamp,
            attendees,
            notes,
        })
    }
}
