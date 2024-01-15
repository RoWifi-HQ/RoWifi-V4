use rowifi_database::{postgres::types::Json, Database};
use rowifi_models::{
    audit_log::{AuditLog, AuditLogData, AuditLogKind},
    bind::{Assetbind, BindType},
    id::{GuildId, UserId},
    roblox::id::AssetId,
};
use std::collections::{HashMap, HashSet};
use time::OffsetDateTime;

use crate::error::RoError;

#[derive(Debug)]
pub struct DeleteAssetbind {
    pub deleted: u32,
    pub invalid: Vec<AssetId>,
}

pub async fn delete_assetbinds(
    database: &Database,
    assetbinds: &[Assetbind],
    guild_id: GuildId,
    author_id: UserId,
    args: Vec<AssetId>,
) -> Result<DeleteAssetbind, RoError> {
    let mut map = HashMap::new();
    for (idx, assetbind) in assetbinds.iter().enumerate() {
        map.insert(assetbind.asset_id, idx);
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
        return Ok(DeleteAssetbind {
            deleted: 0,
            invalid,
        });
    }

    let new_assetbinds = assetbinds
        .iter()
        .filter(|r| binds_to_delete.contains(&r.asset_id))
        .cloned()
        .collect::<Vec<_>>();
    let rows = database
        .execute(
            "UPDATE guilds SET assetbinds = $2 WHERE guild_id = $1",
            &[&guild_id, &Json(new_assetbinds)],
        )
        .await?;

    let log = AuditLog {
        kind: AuditLogKind::BindDelete,
        guild_id: Some(guild_id),
        user_id: Some(author_id),
        timestamp: OffsetDateTime::now_utc(),
        metadata: AuditLogData::BindDelete {
            count: rows as i32,
            kind: BindType::Asset,
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

    Ok(DeleteAssetbind {
        deleted: rows as u32,
        invalid,
    })
}
