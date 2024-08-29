use rowifi_database::{postgres::types::Json, Database};
use rowifi_models::{
    audit_log::{AuditLog, AuditLogData, AuditLogKind},
    bind::{BindType, Custombind, Template},
    discord::cache::CachedRole,
    id::{GuildId, RoleId, UserId},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Add};
use time::OffsetDateTime;

use super::parser::parser;
use crate::error::RoError;

#[derive(Debug, Serialize)]
pub struct AddCustombind {
    pub bind: Custombind,
    pub ignored_roles: Vec<RoleId>,
}

#[derive(Debug)]
pub enum AddCustombindError {
    Code(String),
    Other(RoError),
}

#[derive(Debug, Deserialize)]
pub struct CustombindArguments {
    pub code: String,
    pub template: Template,
    pub priority: Option<i32>,
    pub discord_roles: Vec<RoleId>,
}

/// Adds a custombind to the server. Validates the discord roles if they exist and are not managed.
///
/// # Errors
///
/// See [`RoError`] for details.
pub async fn add_custombind(
    database: &Database,
    guild_id: GuildId,
    author_id: UserId,
    existing_custombinds: &[Custombind],
    server_roles: &HashMap<RoleId, CachedRole>,
    args: CustombindArguments,
) -> Result<AddCustombind, AddCustombindError> {
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

    // TODO: Validate the custombind code
    if let Err(err) = parser(&args.code) {
        return Err(AddCustombindError::Code(err.to_string()));
    }

    let bind = Json(Custombind {
        custom_bind_id: existing_custombinds
            .iter()
            .map(|c| c.custom_bind_id)
            .max()
            .unwrap_or_default()
            .add(1),
        code: args.code,
        discord_roles: roles_to_add,
        priority: args.priority.unwrap_or_default(),
        template: args.template,
    });

    database
        .execute(
            "UPDATE guilds SET custombinds = custombinds || $2::jsonb WHERE guild_id = $1",
            &[&guild_id, &bind],
        )
        .await
        .map_err(|err| AddCustombindError::Other(err.into()))?;

    let log = AuditLog {
        kind: AuditLogKind::BindCreate,
        guild_id: Some(guild_id),
        user_id: Some(author_id),
        timestamp: OffsetDateTime::now_utc(),
        metadata: AuditLogData::BindCreate {
            count: 1,
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
        .await
        .map_err(|err| AddCustombindError::Other(err.into()))?;

    Ok(AddCustombind {
        bind: bind.0,
        ignored_roles,
    })
}
