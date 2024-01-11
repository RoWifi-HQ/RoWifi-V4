use std::collections::HashMap;

use rowifi_framework::prelude::*;
use rowifi_models::{
    audit_log::{AuditLog, AuditLogData, AuditLogKind},
    bind::{BindType, Rankbind, Template},
    discord::{
        http::interaction::{InteractionResponse, InteractionResponseType},
        util::Timestamp,
    },
    id::RoleId,
    roblox::id::GroupId,
};
use twilight_mention::Mention;

#[derive(Arguments)]
pub struct RankbindArguments {
    pub group_id: u64,
    pub rank_id: u8,
    pub template: String,
    pub priority: Option<i32>,
    pub discord_roles: Vec<RoleId>,
}

pub async fn new_rankbind(
    bot: Extension<BotContext>,
    command: Command<RankbindArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = new_rankbind_func(bot, command.ctx, command.args).await {
            tracing::error!(?err);
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

async fn new_rankbind_func(
    bot: Extension<BotContext>,
    ctx: CommandContext,
    args: RankbindArguments,
) -> CommandResult {
    let guild = bot
        .get_guild(
            "SELECT guild_id, rankbinds FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;
    let server = bot.server(ctx.guild_id).await?;

    // Get all the group's ranks so we can check if the rank id provided exists.
    let Some(ranks) = bot.roblox.get_group_ranks(GroupId(args.group_id)).await? else {
        let message = format!(
            r#"
Oh no! There does not seem to be a group with ID {}
        "#,
            args.group_id
        );
        ctx.respond(&bot).content(&message).unwrap().exec().await?;
        return Ok(());
    };
    tracing::debug!(?ranks);

    let Some(rank) = ranks.iter().find(|r| r.rank == args.rank_id as u32) else {
        let message = format!(
            r#"
Oh no! There does not seem to be a rank with ID {} in the group {}
        "#,
            args.rank_id, args.group_id
        );
        ctx.respond(&bot).content(&message).unwrap().exec().await?;
        return Ok(());
    };

    let server_roles = bot
        .cache
        .guild_roles(server.roles.iter().copied())
        .await?
        .into_iter()
        .map(|r| (r.id, r))
        .collect::<HashMap<_, _>>();
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

    let bind = rowifi_database::postgres::types::Json(Rankbind {
        group_id: GroupId(args.group_id),
        discord_roles: roles_to_add,
        group_rank_id: args.rank_id as u32,
        roblox_rank_id: rank.id.clone(),
        priority: args.priority.unwrap_or_default(),
        template: Template(args.template),
    });

    let idx = guild
        .rankbinds
        .0
        .iter()
        .position(|r| r.roblox_rank_id == bind.0.roblox_rank_id);

    bot.database
        .execute(
            &format!(
                "UPDATE guilds SET rankbinds[{}] = $2 WHERE guild_id = $1",
                idx.unwrap_or_else(|| guild.rankbinds.0.len())
            ),
            &[&ctx.guild_id, &bind],
        )
        .await?;

    let mut description = String::new();
    if idx.is_some() {
        description.push_str(":warning: Bind already exists. Modified it to:\n\n")
    }
    description.push_str(&format!("**Rank Id: {}**\n", bind.0.group_rank_id));
    description.push_str(&format!(
        "Template: {}\nPriority: {}\n Roles: {}",
        bind.0.template,
        bind.0.priority,
        bind.0
            .discord_roles
            .into_iter()
            .map(|r| r.0.mention().to_string())
            .collect::<String>()
    ));

    if !ignored_roles.is_empty() {
        let ignored_roles_str = ignored_roles
            .iter()
            .map(|r| r.0.mention().to_string())
            .collect::<String>();
        description.push_str(&format!("\n🚫 Invalid Roles: {}", ignored_roles_str));
    }

    let log = AuditLog {
        kind: AuditLogKind::BindCreate,
        guild_id: Some(ctx.guild_id),
        user_id: Some(ctx.author_id),
        timestamp: OffsetDateTime::now_utc(),
        metadata: AuditLogData::BindCreate {
            count: 1,
            kind: BindType::Rank,
        },
    };

    bot.database
        .execute(
            r#"INSERT INTO audit_logs(kind, guild_id, user_id, timestamp, metadata) 
        VALUES($1, $2, $3, $4, $5)"#,
            &[
                &log.kind,
                &log.guild_id,
                &log.user_id,
                &log.timestamp,
                &rowifi_database::postgres::types::Json(log.metadata),
            ],
        )
        .await?;

    let embed = EmbedBuilder::new()
        .color(DARK_GREEN)
        .footer(EmbedFooterBuilder::new("RoWifi").build())
        .timestamp(Timestamp::from_secs(OffsetDateTime::now_utc().unix_timestamp()).unwrap())
        .title("Bind Addition Successful")
        .description(description)
        .build();
    ctx.respond(&bot).embeds(&[embed]).unwrap().exec().await?;

    Ok(())
}
