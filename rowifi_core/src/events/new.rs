use chrono::Utc;
use rowifi_database::{
    postgres::{types::Json, Row},
    Database,
};
use rowifi_models::{
    audit_log::{AuditLog, AuditLogData, AuditLogKind, EventLog as AuditEventLog},
    events::{EventLog, EventType},
    id::{GuildId, UserId},
    roblox::id::UserId as RobloxUserId,
};

use crate::error::RoError;

pub struct EventLogArguments {
    pub host_id: RobloxUserId,
    pub event_type: u32,
    pub attendees: Vec<RobloxUserId>,
    pub notes: Option<String>,
}

pub enum EventLogError {
    InvalidEventType,
    Other(RoError),
}

pub struct EventLogRow {
    pub guild_event_id: i64,
}

/// Logs an event for the server. Also checks if the attendees are valid.
///
/// # Errors
/// See [`RoError`] for details.
#[allow(clippy::missing_panics_doc, clippy::cast_possible_wrap)]
pub async fn log_event(
    database: &Database,
    guild_id: GuildId,
    author_id: UserId,
    event_types: &[EventType],
    args: EventLogArguments,
) -> Result<EventLog, EventLogError> {
    if !event_types.iter().any(|e| e.id == args.event_type) {
        return Err(EventLogError::InvalidEventType);
    };

    let mut new_event = EventLog {
        guild_id,
        event_type: args.event_type as i32,
        guild_event_id: 0,
        host_id: args.host_id,
        timestamp: Utc::now(),
        attendees: args.attendees,
        notes: args.notes,
    };

    let row = database.query_opt::<EventLogRow>(
        r"INSERT INTO events(guild_id, event_type, guild_event_id, host_id, timestamp, attendees, notes)
        VALUES($1, $2, (SELECT COALESCE(max(guild_event_id) + 1, 1) FROM events WHERE guild_id = $1), $3, $4, $5, $6)
        RETURNING guild_event_id",
        &[&new_event.guild_id, &new_event.event_type, &new_event.host_id, &new_event.timestamp, &new_event.attendees, &new_event.notes]
    ).await.map_err(RoError::from)?;

    new_event.guild_event_id = row.unwrap().guild_event_id;

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let log = AuditLog {
        kind: AuditLogKind::BindDelete,
        guild_id: Some(guild_id),
        user_id: Some(author_id),
        timestamp: Utc::now(),
        metadata: AuditLogData::EventLog(AuditEventLog {
            guild_event_id: new_event.guild_event_id,
        }),
    };

    database
        .execute(
            r"INSERT INTO audit_logs(kind, guild_id, user_id, timestamp, metadata) 
        VALUES($1, $2, $3, $4, $5)",
            &[
                &log.kind,
                &log.guild_id,
                &log.user_id,
                &log.timestamp,
                &Json(log.metadata),
            ],
        )
        .await
        .map_err(RoError::from)?;

    Ok(new_event)
}

impl TryFrom<Row> for EventLogRow {
    type Error = rowifi_database::postgres::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        let guild_event_id = row.try_get("guild_event_id")?;
        Ok(Self { guild_event_id })
    }
}

impl From<RoError> for EventLogError {
    fn from(err: RoError) -> Self {
        EventLogError::Other(err)
    }
}
