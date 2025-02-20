use chrono::Utc;
use rowifi_database::{postgres::types::Json, Database};
use rowifi_models::{
    audit_log::{AuditLog, AuditLogData, AuditLogKind, EventTypeCreate},
    events::EventType,
    id::{GuildId, UserId},
};
use serde::{Deserialize, Serialize};

use crate::error::RoError;

#[derive(Debug, Serialize)]
pub struct AddEventType {
    pub event: EventType,
}

#[derive(Debug, Deserialize)]
pub struct EventTypeArguments {
    pub id: u32,
    pub name: String,
}

#[derive(Debug)]
pub enum AddEventTypeError {
    IdAlreadyExists,
    Generic(RoError),
}

/// Adds an event type to the server.
///
/// # Errors
///
/// See [`RoError`] for details.
pub async fn add_event_type(
    database: &Database,
    guild_id: GuildId,
    author_id: UserId,
    mut existing_event_types: Vec<EventType>,
    args: EventTypeArguments,
) -> Result<AddEventType, AddEventTypeError> {
    if existing_event_types.iter().any(|e| e.id == args.id) {
        return Err(AddEventTypeError::IdAlreadyExists);
    }

    let new_event_type = EventType {
        id: args.id,
        name: args.name,
        disabled: false,
    };
    existing_event_types.push(new_event_type.clone());

    database
        .execute(
            "UPDATE guilds SET event_types = $2 WHERE guild_id = $1",
            &[&guild_id, &Json(existing_event_types)],
        )
        .await
        .map_err(RoError::from)?;

    let log = AuditLog {
        kind: AuditLogKind::EventTypeCreate,
        guild_id: Some(guild_id),
        user_id: Some(author_id),
        timestamp: Utc::now(),
        metadata: AuditLogData::EventTypeCreate(EventTypeCreate { id: args.id }),
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

    Ok(AddEventType {
        event: new_event_type,
    })
}

impl From<RoError> for AddEventTypeError {
    fn from(err: RoError) -> Self {
        Self::Generic(err)
    }
}
