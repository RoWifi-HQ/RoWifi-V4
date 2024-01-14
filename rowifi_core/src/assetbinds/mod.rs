use std::collections::HashMap;
use rowifi_database::{Database, postgres::types::Json};
use rowifi_models::{
    bind::{AssetType, Assetbind, Template, BindType},
    discord::cache::CachedRole,
    id::{GuildId, RoleId, UserId},
    roblox::id::AssetId, audit_log::{AuditLog, AuditLogKind, AuditLogData},
};
use time::OffsetDateTime;

use crate::error::RoError;

#[derive(Debug)]
pub struct AddAssetbind {
    pub bind: Assetbind,
    pub ignored_roles: Vec<RoleId>,
    pub modified: bool,
}

#[derive(Debug)]
pub enum AddAssetbindError {
    Generic(RoError),
}

#[derive(Debug)]
pub struct AssetbindArguments {
    pub kind: AssetType,
    pub asset_id: AssetId,
    pub template: Template,
    pub priority: Option<i32>,
    pub discord_roles: Vec<RoleId>,
}

pub async fn add_assetbind(
    database: &Database,
    guild_id: GuildId,
    author_id: UserId,
    existing_assetbinds: &[Assetbind],
    server_roles: &HashMap<RoleId, CachedRole>,
    args: AssetbindArguments,
) -> Result<AddAssetbind, AddAssetbindError> {
    // TODO: Check for a way to validate an asset
    let mut ignored_roles = Vec::new();
    let mut roles_to_add = Vec::new();
    // Check if the discord roles provided exist or if they are some integration's roles.
    for role in args.discord_roles {
        if let Some(server_role) = server_roles.get(&role) {
            if server_role.managed {
                ignored_roles.push(role);
            } else {
                roles_to_add.push(role);
            }
        }
    }

    let bind = Json(Assetbind {
        asset_type: args.kind,
        asset_id: args.asset_id,
        discord_roles: roles_to_add,
        priority: args.priority.unwrap_or_default(),
        template: args.template,
    });

    let idx = existing_assetbinds
        .iter()
        .position(|r| r.asset_id == bind.0.asset_id);

    database
        .execute(
            &format!(
                "UPDATE guilds SET assetbinds[{}] = $2 WHERE guild_id = $1",
                idx.unwrap_or_else(|| existing_assetbinds.len())
            ),
            &[&guild_id, &bind],
        )
        .await
        .map_err(|err| RoError::from(err))?;

    let log = AuditLog {
        kind: AuditLogKind::BindCreate,
        guild_id: Some(guild_id),
        user_id: Some(author_id),
        timestamp: OffsetDateTime::now_utc(),
        metadata: AuditLogData::BindCreate {
            count: 1,
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
        .await
        .map_err(|err| RoError::from(err))?;

    Ok(AddAssetbind {
        bind: bind.0,
        ignored_roles,
        modified: idx.is_some(),
    })
}

impl From<RoError> for AddAssetbindError {
    fn from(err: RoError) -> Self {
        Self::Generic(err)
    }
}
