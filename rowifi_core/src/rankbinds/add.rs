use chrono::Utc;
use rowifi_database::{postgres::types::Json, Database};
use rowifi_models::{
    audit_log::{AuditLog, AuditLogData, AuditLogKind},
    bind::{BindType, Rankbind, Template},
    discord::cache::CachedRole,
    id::{GuildId, RoleId, UserId},
    roblox::id::GroupId,
};
use rowifi_roblox::RobloxClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::RoError;

#[derive(Debug, Serialize)]
pub struct AddRankbind {
    pub bind: Rankbind,
    pub ignored_roles: Vec<RoleId>,
    pub modified: bool,
}

#[derive(Debug)]
pub enum AddRankbindError {
    InvalidGroup,
    InvalidRank,
    Generic(RoError),
}

#[derive(Debug, Deserialize)]
pub struct RankbindArguments {
    pub group_id: GroupId,
    pub rank_id: u32,
    pub template: Template,
    pub priority: Option<i32>,
    pub discord_roles: Vec<RoleId>,
}

/// Adds a rankbind to the server. Modifies it if the rankbind already exists.
/// Validates the discord roles if they exist and are not managed.
///
/// # Errors
///
/// See [`AddRankbindError`] for details.
pub async fn add_rankbind(
    roblox: &RobloxClient,
    database: &Database,
    guild_id: GuildId,
    author_id: UserId,
    existing_rankbinds: &[Rankbind],
    server_roles: &HashMap<RoleId, CachedRole>,
    args: RankbindArguments,
) -> Result<AddRankbind, AddRankbindError> {
    let Some(ranks) = roblox
        .get_group_ranks(args.group_id)
        .await
        .map_err(RoError::from)?
    else {
        return Err(AddRankbindError::InvalidGroup);
    };

    let Some(rank) = ranks.iter().find(|r| r.rank == u32::from(args.rank_id)) else {
        return Err(AddRankbindError::InvalidRank);
    };

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

    let bind = Json(Rankbind {
        group_id: args.group_id,
        discord_roles: roles_to_add,
        group_rank_id: u32::from(args.rank_id),
        roblox_rank_id: rank.id.clone(),
        priority: args.priority.unwrap_or_default(),
        template: args.template,
    });

    let idx = existing_rankbinds
        .iter()
        .position(|r| r.roblox_rank_id == bind.0.roblox_rank_id);

    database
        .execute(
            &format!(
                "UPDATE guilds SET rankbinds[{}] = $2 WHERE guild_id = $1",
                idx.unwrap_or(existing_rankbinds.len())
            ),
            &[&guild_id, &bind],
        )
        .await
        .map_err(RoError::from)?;

    let log = AuditLog {
        kind: AuditLogKind::BindCreate,
        guild_id: Some(guild_id),
        user_id: Some(author_id),
        timestamp:Utc::now(),
        metadata: AuditLogData::BindCreate {
            count: 1,
            kind: BindType::Rank,
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
        .await
        .map_err(RoError::from)?;

    Ok(AddRankbind {
        bind: bind.0,
        ignored_roles,
        modified: idx.is_some(),
    })
}

impl From<RoError> for AddRankbindError {
    fn from(err: RoError) -> Self {
        AddRankbindError::Generic(err)
    }
}
