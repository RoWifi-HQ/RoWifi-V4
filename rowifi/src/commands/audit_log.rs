use rowifi_database::postgres::types::ToSql;
use rowifi_framework::prelude::*;
use rowifi_models::{
    audit_log::{AuditLog, AuditLogData, AuditLogKind},
    discord::{
        http::interaction::{InteractionResponse, InteractionResponseType},
        util::Timestamp,
    },
    id::UserId,
};
use std::collections::HashMap;

#[derive(Arguments, Debug)]
pub struct AuditLogArguments {
    pub page: u32,
    pub user: Option<UserId>,
    pub action: Option<AuditLogKind>,
}

pub async fn audit_logs(
    bot: Extension<BotContext>,
    command: Command<AuditLogArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = audit_logs_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

pub async fn audit_logs_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: AuditLogArguments,
) -> CommandResult {
    let items = args.page.saturating_sub(1) * 100;
    let (statement, params): (_, Vec<&(dyn ToSql + Sync)>) = match (&args.user, &args.action) {
        (Some(user), Some(action)) => (
            format!(
                "SELECT * FROM audit_logs WHERE guild_id = $1 AND user_id = $2 AND kind = $3 ORDER BY timestamp LIMIT 100 OFFSET {}",
                items
            ), 
            vec![&ctx.guild_id, user, action]
        ),
        (Some(user), None) => (
            format!(
                "SELECT * FROM audit_logs WHERE guild_id = $1 AND user_id = $2 ORDER BY timestamp LIMIT 100 OFFSET {}",
                items
            ), 
            vec![&ctx.guild_id, user]
        ),
        (None, Some(action)) => (
            format!(
                "SELECT * FROM audit_logs WHERE guild_id = $1 AND action = $2 ORDER BY timestamp LIMIT 100 OFFSET {}",
                items
            ), 
            vec![&ctx.guild_id, action]
        ),
        (None, None) => (
            format!(
                "SELECT * FROM audit_logs WHERE guild_id = $1 ORDER BY timestamp LIMIT 100 OFFSET {}",
                items
            ), vec![&ctx.guild_id]
        )
    };

    let audit_logs = bot.database.query::<AuditLog>(&statement, &params).await?;

    let user_ids = audit_logs.iter().filter_map(|a| a.user_id);
    let users = bot
        .cache
        .guild_members(ctx.guild_id, user_ids)
        .await?
        .into_iter()
        .map(|u| (u.id, u))
        .collect::<HashMap<_, _>>();

    let roblox_user_ids = audit_logs.iter().filter_map(|a| match &a.metadata {
        AuditLogData::XPAdd(xp) => Some(xp.target_roblox_user),
        AuditLogData::XPRemove(xp) => Some(xp.target_roblox_user),
        AuditLogData::XPSet(xp) => Some(xp.target_roblox_user),
        AuditLogData::SetRank(setrank) => Some(setrank.target_roblox_user),
        _ => None,
    });
    let roblox_users = bot
        .roblox
        .get_users(roblox_user_ids)
        .await?
        .into_iter()
        .map(|u| (u.id, u))
        .collect::<HashMap<_, _>>();

    let mut description = String::new();
    for audit_log in audit_logs {
        let user = audit_log
            .user_id
            .and_then(|u| users.get(&u))
            .map(|u| u.nickname.clone().unwrap_or_else(|| u.username.clone()))
            .unwrap_or_else(|| audit_log.user_id.map(|u| u.to_string()).unwrap_or_default());
        match audit_log.metadata {
            AuditLogData::BindCreate(bind) => {
                description.push_str(&format!(
                    "- {} created {} {}bind(s)",
                    user, bind.count, bind.kind
                ));
            }
            AuditLogData::BindModify(bind) => {
                description.push_str(&format!(
                    "- {} modified {} {}bind(s)",
                    user, bind.count, bind.kind
                ));
            }
            AuditLogData::BindDelete(bind) => {
                description.push_str(&format!(
                    "- {} deleted {} {}bind(s)",
                    user, bind.count, bind.kind
                ));
            }
            AuditLogData::XPAdd(xp) => {
                let target_user = roblox_users
                    .get(&xp.target_roblox_user)
                    .map(|u| u.name.clone())
                    .unwrap_or_else(|| xp.target_roblox_user.to_string());
                description.push_str(&format!("- {} added {} XP to {}", user, xp.xp, target_user));
            }
            AuditLogData::XPRemove(xp) => {
                let target_user = roblox_users
                    .get(&xp.target_roblox_user)
                    .map(|u| u.name.clone())
                    .unwrap_or_else(|| xp.target_roblox_user.to_string());
                description.push_str(&format!(
                    "- {} removed {} XP from {}",
                    user, xp.xp, target_user
                ));
            }
            AuditLogData::SetRank(set_rank) => {
                let target_user = roblox_users
                    .get(&set_rank.target_roblox_user)
                    .map(|u| u.name.clone())
                    .unwrap_or_else(|| set_rank.target_roblox_user.to_string());
                description.push_str(&format!(
                    "- {} set {}'s rank in {} to {}",
                    user, target_user, set_rank.group_id, set_rank.group_rank_id
                ));
            }
            AuditLogData::XPSet(xp) => {
                let target_user = roblox_users
                    .get(&xp.target_roblox_user)
                    .map(|u| u.name.clone())
                    .unwrap_or_else(|| xp.target_roblox_user.to_string());
                description.push_str(&format!("- {} set {}'s XP to {}", user, target_user, xp.xp));
            }
            AuditLogData::DenylistCreate(denylist) => {
                description.push_str(&format!("- {} created a {} denylist", user, denylist.kind));
            }
            AuditLogData::DenylistDelete(denylist) => {
                description.push_str(&format!("- {} deleted {} denylist(s)", user, denylist.count));
            }
            AuditLogData::EventLog(_) => {
                description.push_str(&format!("- {} logged an event", user));
            }
            AuditLogData::SettingModify(setting) => {
                description.push_str(&format!(
                    "- {} modified {} to {}",
                    user, setting.setting, setting.value
                ));
            }
            AuditLogData::EventTypeCreate(_) => {
                description.push_str(&format!("- {} created an Event Type", user));
            }
            AuditLogData::EventTypeModify(_) => {
                description.push_str(&format!("- {} modified an Event Type", user));
            }
            AuditLogData::GroupAccept(group) => {
                let target_user = roblox_users
                    .get(&group.target_roblox_user)
                    .map(|u| u.name.clone())
                    .unwrap_or_else(|| group.target_roblox_user.to_string());
                description.push_str(&format!(
                    "- {} accepted {} to {}",
                    user, target_user, group.group_id
                ));
            }
            AuditLogData::GroupDecline(group) => {
                let target_user = roblox_users
                    .get(&group.target_roblox_user)
                    .map(|u| u.name.clone())
                    .unwrap_or_else(|| group.target_roblox_user.to_string());
                description.push_str(&format!(
                    "- {} declined {}'s join request to {}",
                    user, target_user, group.group_id
                ));
            }
        }
        description.push_str("\n");
    }

    let embed = EmbedBuilder::new()
        .color(BLUE)
        .footer(EmbedFooterBuilder::new("RoWifi").build())
        .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
        .title(format!("Audit Logs | Page {}", args.page))
        .description(description)
        .build();
    ctx.respond(&bot).embeds(&[embed]).unwrap().await?;

    Ok(())
}
