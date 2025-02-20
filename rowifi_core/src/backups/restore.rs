use rowifi_cache::Cache;
use rowifi_database::{postgres::types::Json, Database};
use rowifi_models::{
    backup::BackupGuild,
    bind::{Assetbind, Custombind, Groupbind, Rankbind},
    guild::BypassRole,
    id::{GuildId, RoleId, UserId},
};
use std::collections::{HashMap, HashSet};
use twilight_http::Client as TwilightClient;

use crate::error::RoError;

pub struct BackupArguments {
    pub name: String,
}

pub enum BackupError {
    NotFound,
    Other(RoError),
}

pub struct BackupGuildRow {
    pub data: Json<BackupGuild>,
}

#[allow(clippy::too_many_lines)]
pub async fn restore_backup(
    database: &Database,
    cache: &Cache,
    http: &TwilightClient,
    author: UserId,
    args: BackupArguments,
    guild_id: GuildId,
) -> Result<(), BackupError> {
    let backup_guild = database
        .query_opt::<BackupGuildRow>(
            "SELECT data FROM backups WHERE user_id = $1 and name = $2",
            &[&author, &args.name],
        )
        .await
        .map_err(|err| BackupError::Other(err.into()))?;
    let backup_guild = if let Some(b) = backup_guild {
        b.data.0
    } else {
        return Err(BackupError::NotFound);
    };

    // Collect all unique roles
    let mut all_roles = HashSet::new();
    all_roles.extend(
        backup_guild
            .rankbinds
            .iter()
            .flat_map(|r| r.discord_roles.clone()),
    );
    all_roles.extend(
        backup_guild
            .groupbinds
            .iter()
            .flat_map(|r| r.discord_roles.clone()),
    );
    all_roles.extend(
        backup_guild
            .custombinds
            .iter()
            .flat_map(|r| r.discord_roles.clone()),
    );
    all_roles.extend(
        backup_guild
            .assetbinds
            .iter()
            .flat_map(|r| r.discord_roles.clone()),
    );
    all_roles.extend(backup_guild.unverified_roles.clone());
    all_roles.extend(backup_guild.verified_roles.clone());
    all_roles.extend(backup_guild.bypass_roles.iter().map(|b| b.role.clone()));

    let server = cache
        .guild(guild_id)
        .await
        .map_err(|err| BackupError::Other(err.into()))?
        .unwrap();
    let guild_roles = cache
        .guild_roles(server.roles.into_iter())
        .await
        .map_err(|err| BackupError::Other(err.into()))?
        .into_iter()
        .map(|r| (r.name, r.id))
        .collect::<HashMap<_, _>>();

    // Create roles if they don't exist
    let mut all_roles_map = HashMap::new();
    for r in all_roles {
        if let Some(existing) = guild_roles.get(&r) {
            all_roles_map.insert(r, *existing);
        } else {
            let role = http
                .create_role(guild_id.0)
                .name(&r)
                .await
                .map_err(|err| BackupError::Other(err.into()))?
                .model()
                .await
                .map_err(|err| BackupError::Other(err.into()))?;
            all_roles_map.insert(r, RoleId(role.id));
        }
    }

    let unverified_roles = backup_guild
        .unverified_roles
        .iter()
        .filter_map(|r| all_roles_map.get(r.as_str()).copied())
        .collect::<Vec<_>>();
    let verified_roles = backup_guild
        .verified_roles
        .iter()
        .filter_map(|r| all_roles_map.get(r.as_str()).copied())
        .collect::<Vec<_>>();
    let rankbinds = backup_guild
        .rankbinds
        .iter()
        .map(|b| Rankbind {
            group_id: b.group_id,
            group_rank_id: b.group_rank_id,
            roblox_rank_id: b.roblox_rank_id.clone(),
            priority: b.priority,
            template: b.template.clone(),
            discord_roles: b
                .discord_roles
                .iter()
                .filter_map(|r| all_roles_map.get(r.as_str()).copied())
                .collect(),
        })
        .collect::<Vec<_>>();
    let groupbinds = backup_guild
        .groupbinds
        .iter()
        .map(|b| Groupbind {
            group_id: b.group_id,
            priority: b.priority,
            template: b.template.clone(),
            discord_roles: b
                .discord_roles
                .iter()
                .filter_map(|r| all_roles_map.get(r.as_str()).copied())
                .collect(),
        })
        .collect::<Vec<_>>();
    let custombinds = backup_guild
        .custombinds
        .iter()
        .map(|b| Custombind {
            custom_bind_id: b.custom_bind_id,
            code: b.code.clone(),
            priority: b.priority,
            template: b.template.clone(),
            discord_roles: b
                .discord_roles
                .iter()
                .filter_map(|r| all_roles_map.get(r.as_str()).copied())
                .collect(),
        })
        .collect::<Vec<_>>();
    let assetbinds = backup_guild
        .assetbinds
        .iter()
        .map(|b| Assetbind {
            asset_id: b.asset_id,
            asset_type: b.asset_type,
            priority: b.priority,
            template: b.template.clone(),
            discord_roles: b
                .discord_roles
                .iter()
                .filter_map(|r| all_roles_map.get(r.as_str()).copied())
                .collect(),
        })
        .collect::<Vec<_>>();
    let bypass_roles = backup_guild
        .bypass_roles
        .iter()
        .filter_map(|b| {
            all_roles_map.get(&b.role).map(|r| BypassRole {
                role_id: *r,
                kind: b.kind,
            })
        })
        .collect::<Vec<_>>();

    database.execute(
        "UPDATE guilds SET bypass_roles = $1, unverified_roles = $2, verified_roles = $3, rankbinds = $4, groupbinds = $5, assetbinds = $6, custombinds = $7, xp_binds = $8, deny_lists = $9, default_template = $10, update_on_join = $11, event_types = $12, auto_detection = $13, sync_xp_on_setrank = $14 WHERE guild_id = $15", 
        &[
            &Json(bypass_roles),
            &unverified_roles,
            &verified_roles,
            &Json(rankbinds),
            &Json(groupbinds),
            &Json(custombinds),
            &Json(assetbinds),
            &Json(backup_guild.xp_binds),
            &Json(backup_guild.deny_lists),
            &backup_guild.default_template,
            &backup_guild.update_on_join,
            &Json(backup_guild.event_types),
            &backup_guild.auto_detection,
            &backup_guild.sync_xp_on_setrank,
            &guild_id
        ])
    .await
    .map_err(|err| BackupError::Other(err.into()))?;

    Ok(())
}

impl TryFrom<rowifi_database::postgres::Row> for BackupGuildRow {
    type Error = rowifi_database::postgres::Error;

    fn try_from(row: rowifi_database::postgres::Row) -> Result<Self, Self::Error> {
        let data = row.try_get("data")?;

        Ok(Self { data })
    }
}
