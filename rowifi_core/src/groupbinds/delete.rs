use rowifi_database::{postgres::types::Json, Database};
use rowifi_models::{
    audit_log::{AuditLog, AuditLogData, AuditLogKind},
    bind::{BindType, Groupbind},
    id::{GuildId, UserId},
    roblox::id::GroupId,
};
use std::collections::{HashMap, HashSet};
use time::OffsetDateTime;

use crate::error::RoError;

#[derive(Debug)]
pub struct DeleteGroupbind {
    pub deleted: u32,
    pub invalid: Vec<GroupId>,
}

/// Deletes a list of groupbinds from the server.
///
/// # Errors
///
/// See [`RoError`] for details.
pub async fn delete_groupbinds(
    database: &Database,
    groupbinds: &[Groupbind],
    guild_id: GuildId,
    author_id: UserId,
    args: Vec<GroupId>,
) -> Result<DeleteGroupbind, RoError> {
    let mut map = HashMap::new();
    for (idx, groupbind) in groupbinds.iter().enumerate() {
        map.insert(groupbind.group_id, idx);
    }

    let mut binds_to_delete = HashSet::new();
    let mut invalid = Vec::new();
    for arg in args {
        if map.contains_key(&arg) {
            binds_to_delete.insert(arg);
        } else {
            invalid.push(arg);
        }
    }

    if binds_to_delete.is_empty() {
        return Ok(DeleteGroupbind {
            deleted: 0,
            invalid,
        });
    }

    let new_groupbinds = groupbinds
        .iter()
        .filter(|r| binds_to_delete.contains(&r.group_id))
        .cloned()
        .collect::<Vec<_>>();
    database
        .execute(
            "UPDATE guilds SET groupbinds = $2 WHERE guild_id = $1",
            &[&guild_id, &Json(new_groupbinds)],
        )
        .await?;

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let log = AuditLog {
        kind: AuditLogKind::BindDelete,
        guild_id: Some(guild_id),
        user_id: Some(author_id),
        timestamp: OffsetDateTime::now_utc(),
        metadata: AuditLogData::BindDelete {
            count: binds_to_delete.len() as i32,
            kind: BindType::Group,
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
    Ok(DeleteGroupbind {
        deleted: binds_to_delete.len() as u32,
        invalid,
    })
}
