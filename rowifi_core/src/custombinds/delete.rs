use chrono::Utc;
use rowifi_database::{postgres::types::Json, Database};
use rowifi_models::{
    audit_log::{AuditLog, AuditLogData, AuditLogKind},
    bind::{BindType, Custombind},
    id::{GuildId, UserId},
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::error::RoError;

#[derive(Debug, Serialize)]
pub struct DeleteCustombind {
    pub deleted: u32,
    pub invalid: Vec<CustombindArguments>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CustombindArguments {
    pub custom_bind_id: u32,
}

/// Deletes a list of rankbinds from the server.
///
/// # Errors
///
/// See [`RoError`] for details.
pub async fn delete_custombinds(
    database: &Database,
    custombinds: &[Custombind],
    guild_id: GuildId,
    author_id: UserId,
    args: Vec<CustombindArguments>,
) -> Result<DeleteCustombind, RoError> {
    let mut set = HashSet::new();
    for custombind in custombinds.iter() {
        set.insert(custombind.custom_bind_id);
    }

    let mut binds_to_delete = HashSet::new();
    let mut invalid = Vec::new();
    for arg in args {
        if set.contains(&arg.custom_bind_id) {
            binds_to_delete.insert(arg.custom_bind_id);
        } else {
            invalid.push(arg);
        }
    }

    if binds_to_delete.is_empty() {
        return Ok(DeleteCustombind {
            deleted: 0,
            invalid,
        });
    }

    let new_custombinds = custombinds
        .iter()
        .filter(|c| !binds_to_delete.contains(&c.custom_bind_id))
        .cloned()
        .collect::<Vec<_>>();
    database
        .execute(
            "UPDATE guilds SET custombinds = $2 WHERE guild_id = $1",
            &[&guild_id, &Json(new_custombinds)],
        )
        .await?;

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let log = AuditLog {
        kind: AuditLogKind::BindDelete,
        guild_id: Some(guild_id),
        user_id: Some(author_id),
        timestamp: Utc::now(),
        metadata: AuditLogData::BindDelete {
            count: binds_to_delete.len() as i32,
            kind: BindType::Custom,
        },
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
        .await?;

    #[allow(clippy::cast_possible_truncation)]
    Ok(DeleteCustombind {
        deleted: binds_to_delete.len() as u32,
        invalid,
    })
}
