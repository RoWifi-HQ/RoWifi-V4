use chrono::Utc;
use rowifi_database::{postgres::types::Json, Database};
use rowifi_models::{
    audit_log::{AuditLog, AuditLogData, AuditLogKind, BindCreate},
    bind::{BindType, Groupbind, Template},
    discord::cache::CachedRole,
    id::{GuildId, RoleId, UserId},
    roblox::id::GroupId,
};
use rowifi_roblox::RobloxClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::RoError;

#[derive(Debug, Serialize)]
pub struct AddGroupbind {
    pub bind: Groupbind,
    pub ignored_roles: Vec<RoleId>,
    pub modified: bool,
}

#[derive(Debug)]
pub enum AddGroupbindError {
    InvalidGroup,
    Generic(RoError),
}

#[derive(Debug, Deserialize)]
pub struct GroupbindArguments {
    pub group_id: GroupId,
    pub template: Template,
    pub priority: Option<i32>,
    pub discord_roles: Vec<RoleId>,
}

/// Adds a groupbind to the server. Modifies it if the groupbind already exists.
/// Validates the discord roles if they exist and are not managed.
///
/// # Errors
///
/// See [`AddGroupbindError`] for details.
pub async fn add_groupbind(
    roblox: &RobloxClient,
    database: &Database,
    guild_id: GuildId,
    author_id: UserId,
    mut existing_groupbinds: Vec<Groupbind>,
    server_roles: &HashMap<RoleId, CachedRole>,
    args: GroupbindArguments,
) -> Result<AddGroupbind, AddGroupbindError> {
    if roblox
        .get_group(args.group_id)
        .await
        .map_err(RoError::from)?
        .is_none()
    {
        return Err(AddGroupbindError::InvalidGroup);
    }

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

    let new_bind = Groupbind {
        group_id: args.group_id,
        discord_roles: roles_to_add,
        priority: args.priority.unwrap_or_default(),
        template: args.template,
    };

    let mut modified = false;
    if let Some(bind) = existing_groupbinds
        .iter_mut()
        .find(|r| r.group_id == new_bind.group_id)
    {
        bind.priority = new_bind.priority;
        bind.template = new_bind.template.clone();
        bind.discord_roles.clone_from(&new_bind.discord_roles);
        modified = true;
    } else {
        existing_groupbinds.push(new_bind.clone());
    }

    database
        .execute(
            "UPDATE guilds SET groupbinds = $2 WHERE guild_id = $1",
            &[&guild_id, &Json(existing_groupbinds)],
        )
        .await
        .map_err(RoError::from)?;

    let log = AuditLog {
        kind: AuditLogKind::BindCreate,
        guild_id: Some(guild_id),
        user_id: Some(author_id),
        timestamp: Utc::now(),
        metadata: AuditLogData::BindCreate(BindCreate {
            count: 1,
            kind: BindType::Group,
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

    Ok(AddGroupbind {
        bind: new_bind,
        ignored_roles,
        modified,
    })
}

impl From<RoError> for AddGroupbindError {
    fn from(err: RoError) -> Self {
        Self::Generic(err)
    }
}
