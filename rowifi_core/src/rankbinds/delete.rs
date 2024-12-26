use chrono::Utc;
use rowifi_database::{postgres::types::Json, Database};
use rowifi_models::{
    audit_log::{AuditLog, AuditLogData, AuditLogKind, BindDelete},
    bind::{BindType, Rankbind},
    id::{GuildId, UserId},
    roblox::id::GroupId,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::error::RoError;

#[derive(Debug, Serialize)]
pub struct DeleteRankbind {
    pub deleted: u32,
    pub invalid: Vec<RankbindArguments>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RankbindArguments {
    pub group_id: GroupId,
    pub rank_id: u32,
}

/// Deletes a list of rankbinds from the server.
///
/// # Errors
///
/// See [`RoError`] for details.
pub async fn delete_rankbinds(
    database: &Database,
    rankbinds: &[Rankbind],
    guild_id: GuildId,
    author_id: UserId,
    args: Vec<RankbindArguments>,
) -> Result<DeleteRankbind, RoError> {
    let mut map = HashMap::new();
    for (idx, rankbind) in rankbinds.iter().enumerate() {
        map.insert((rankbind.group_id, rankbind.group_rank_id), idx);
    }

    let mut binds_to_delete = HashSet::new();
    let mut invalid = Vec::new();
    for arg in args {
        if map.contains_key(&(arg.group_id, arg.rank_id)) {
            binds_to_delete.insert((arg.group_id, arg.rank_id));
        } else {
            invalid.push(arg);
        }
    }

    if binds_to_delete.is_empty() {
        return Ok(DeleteRankbind {
            deleted: 0,
            invalid,
        });
    }

    let new_rankbinds = rankbinds
        .iter()
        .filter(|r| !binds_to_delete.contains(&(r.group_id, r.group_rank_id)))
        .cloned()
        .collect::<Vec<_>>();
    database
        .execute(
            "UPDATE guilds SET rankbinds = $2 WHERE guild_id = $1",
            &[&guild_id, &Json(new_rankbinds)],
        )
        .await?;

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let log = AuditLog {
        kind: AuditLogKind::BindDelete,
        guild_id: Some(guild_id),
        user_id: Some(author_id),
        timestamp: Utc::now(),
        metadata: AuditLogData::BindDelete(BindDelete {
            count: binds_to_delete.len() as i32,
            kind: BindType::Rank,
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
        .await?;

    #[allow(clippy::cast_possible_truncation)]
    Ok(DeleteRankbind {
        deleted: binds_to_delete.len() as u32,
        invalid,
    })
}
