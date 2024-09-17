use rowifi_cache::Cache;
use rowifi_database::{postgres::types::Json, Database};
use rowifi_models::{
    backup::{
        BackupAssetbind, BackupBypassRole, BackupCustombind, BackupGroupbind, BackupGuild,
        BackupRankbind,
    },
    guild::PartialRoGuild,
    id::UserId,
};
use std::collections::HashMap;

use crate::error::RoError;

pub struct BackupArguments {
    pub name: String,
    pub author: UserId,
}

pub async fn create_backup(
    database: &Database,
    cache: &Cache,
    guild: PartialRoGuild,
    args: BackupArguments,
) -> Result<(), RoError> {
    let server = cache.guild(guild.guild_id).await?.unwrap();
    let roles = cache
        .guild_roles(server.roles.into_iter())
        .await?
        .into_iter()
        .map(|r| (r.id, r.name))
        .collect::<HashMap<_, _>>();

    let unverified_roles = guild
        .unverified_roles
        .iter()
        .filter_map(|r| roles.get(r).cloned())
        .collect();
    let verified_roles = guild
        .verified_roles
        .iter()
        .filter_map(|r| roles.get(r).cloned())
        .collect();
    let bypass_roles = guild
        .bypass_roles
        .iter()
        .filter_map(|b| {
            roles.get(&b.role_id).map(|r| BackupBypassRole {
                role: r.clone(),
                kind: b.kind,
            })
        })
        .collect();
    let rankbinds = guild
        .rankbinds
        .iter()
        .map(|r| BackupRankbind {
            group_id: r.group_id,
            roblox_rank_id: r.roblox_rank_id.clone(),
            group_rank_id: r.group_rank_id,
            priority: r.priority,
            template: r.template.clone(),
            discord_roles: r
                .discord_roles
                .iter()
                .filter_map(|role| roles.get(role).cloned())
                .collect(),
        })
        .collect();
    let groupbinds = guild
        .groupbinds
        .iter()
        .map(|g| BackupGroupbind {
            group_id: g.group_id,
            priority: g.priority,
            template: g.template.clone(),
            discord_roles: g
                .discord_roles
                .iter()
                .filter_map(|role| roles.get(role).cloned())
                .collect(),
        })
        .collect();
    let custombinds = guild
        .custombinds
        .iter()
        .map(|c| BackupCustombind {
            custom_bind_id: c.custom_bind_id,
            code: c.code.clone(),
            priority: c.priority,
            template: c.template.clone(),
            discord_roles: c
                .discord_roles
                .iter()
                .filter_map(|role| roles.get(role).cloned())
                .collect(),
        })
        .collect();
    let assetbinds = guild
        .assetbinds
        .iter()
        .map(|a| BackupAssetbind {
            asset_id: a.asset_id,
            asset_type: a.asset_type,
            priority: a.priority,
            template: a.template.clone(),
            discord_roles: a
                .discord_roles
                .iter()
                .filter_map(|role| roles.get(role).cloned())
                .collect(),
        })
        .collect();

    let backup = BackupGuild {
        bypass_roles,
        unverified_roles,
        verified_roles,
        rankbinds,
        groupbinds,
        assetbinds,
        custombinds,
        xp_binds: guild.xp_binds.clone(),
        deny_lists: guild.deny_lists.clone(),
        default_template: guild.default_template.unwrap_or_default(),
        update_on_join: guild.update_on_join.unwrap_or_default(),
        event_types: guild.event_types.clone(),
        auto_detection: guild.auto_detection.unwrap_or_default(),
        sync_xp_on_setrank: guild.sync_xp_on_setrank.unwrap_or_default(),
    };
    database
        .execute(
            "INSERT INTO backups(user_id, name, data) VALUES($1, $2, $3) ON CONFLICT(user_id, name) DO UPDATE SET data = $3",
            &[&args.author, &args.name, &Json(backup)],
        )
        .await?;

    Ok(())
}
