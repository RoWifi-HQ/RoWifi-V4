use rowifi_database::{postgres::types::Json, Database};
use rowifi_models::{
    audit_log::{AuditLog, AuditLogData, AuditLogKind},
    deny_list::DenyList,
    id::{GuildId, UserId},
};
use std::collections::{HashMap, HashSet};
use time::OffsetDateTime;

use crate::error::RoError;

#[derive(Debug)]
pub struct DeleteDenylist {
    pub deleted: u32,
    pub invalid: Vec<u32>,
}

pub async fn delete_denylist(
    database: &Database,
    denylists: &[DenyList],
    guild_id: GuildId,
    author_id: UserId,
    args: Vec<u32>,
) -> Result<DeleteDenylist, RoError> {
    let mut map = HashMap::new();
    for (idx, denylist) in denylists.iter().enumerate() {
        map.insert(denylist.id, idx);
    }

    let mut denylists_to_delete = HashSet::new();
    let mut invalid = Vec::new();
    for arg in args {
        if map.contains_key(&arg) {
            denylists_to_delete.insert(arg);
        } else {
            invalid.push(arg);
        }
    }

    if denylists_to_delete.is_empty() {
        return Ok(DeleteDenylist {
            deleted: 0,
            invalid,
        });
    }

    let new_denylists = denylists
        .iter()
        .filter(|d| denylists_to_delete.contains(&d.id))
        .cloned()
        .collect::<Vec<_>>();
    database
        .execute(
            "UPDATE guilds SET denylists = $2 WHERE guild_id = $1",
            &[&guild_id, &Json(new_denylists)],
        )
        .await?;

    let log = AuditLog {
        kind: AuditLogKind::BindDelete,
        guild_id: Some(guild_id),
        user_id: Some(author_id),
        timestamp: OffsetDateTime::now_utc(),
        metadata: AuditLogData::DenylistDelete {
            count: denylists_to_delete.len() as i32,
        },
    };

    database
        .execute(
            r#"INSERT INTO audit_logs(kind, guild_id, user_id, timestamp, metadata) 
            VALUES($1, $2, $3, $4, $5)"#,
            &[
                &log.kind,
                &log.guild_id,
                &log.user_id,
                &log.timestamp,
                &Json(log.metadata),
            ],
        )
        .await?;

    Ok(DeleteDenylist {
        deleted: denylists_to_delete.len() as u32,
        invalid,
    })
}
