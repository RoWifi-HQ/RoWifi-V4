use itertools::Itertools;
use redis::AsyncCommands;
use rowifi_cache::error::CacheError;
use rowifi_core::user::update::{UpdateUser, UpdateUserError};
use rowifi_framework::prelude::*;
use rowifi_models::{
    deny_list::DenyListActionType,
    discord::{
        cache::{CachedGuild, CachedMember},
        http::interaction::{
            InteractionResponse, InteractionResponseData, InteractionResponseType,
        },
        util::Timestamp,
    },
    guild::{BypassRoleKind, GuildType, PartialRoGuild},
    id::{GuildId, RoleId},
    user::RoUser,
};
use serde::Serialize;
use std::{collections::HashSet, fmt::Write, time::Duration};

#[derive(Debug, Serialize)]
pub struct MassUpdateQueueArguments {
    pub guild_id: GuildId,
    pub role_id: Option<RoleId>,
}

pub async fn update_all(bot: Extension<BotContext>, command: Command<()>) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = update_all_func(&bot, &command.ctx).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some("`update-all` queue started".into()),
            ..Default::default()
        }),
    })
}

#[tracing::instrument(skip_all)]
pub async fn update_all_func(bot: &BotContext, ctx: &CommandContext) -> CommandResult {
    let guild = bot.database.query_opt::<PartialRoGuild>("SELECT guild_id, kind, bypass_roles, unverified_roles, verified_roles, rankbinds, groupbinds, custombinds, assetbinds, deny_lists, default_template, log_channel FROM guilds WHERE guild_id = $1", &[&ctx.guild_id]).await?.unwrap_or_else(|| PartialRoGuild::new(ctx.guild_id));
    if guild.kind.unwrap() == GuildType::Free {
        let message = "Mass Update commands are only available to Premium servers";
        ctx.respond(&bot).content(message).unwrap().await?;
    }

    let mut conn = bot.cache.get();
    let _: () = conn
        .publish(
            "update-all",
            serde_json::to_vec(&MassUpdateQueueArguments {
                guild_id: ctx.guild_id,
                role_id: None,
            })
            .unwrap(),
        )
        .await
        .map_err(|err| CacheError::from(err))?;

    tokio::time::sleep(Duration::from_secs(30)).await;
    let server = bot.server(ctx.guild_id).await?;
    let members = bot.cache.guild_members(ctx.guild_id).await?;

    for member in members {
        if let Some(member) = bot.member(ctx.guild_id, member).await? {
            if let Err(err) = update_member(bot, &server, &guild, member).await {
                tracing::error!(err = ?err);
            }
        }
    }

    bot.http
        .create_message(ctx.channel_id.0)
        .content("`update-all` is complete")
        .await?;

    Ok(())
}

#[derive(Arguments, Debug)]
pub struct UpdateRoleArguments {
    // TODO: Change this to RoleId
    pub role: u64,
}

pub async fn update_role(
    bot: Extension<BotContext>,
    command: Command<UpdateRoleArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = update_role_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some("`update-role` queue started".into()),
            ..Default::default()
        }),
    })
}

#[tracing::instrument(skip_all)]
pub async fn update_role_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: UpdateRoleArguments,
) -> CommandResult {
    let guild = bot.database.query_opt::<PartialRoGuild>("SELECT guild_id, kind, bypass_roles, unverified_roles, verified_roles, rankbinds, groupbinds, custombinds, assetbinds, deny_lists, default_template, log_channel FROM guilds WHERE guild_id = $1", &[&ctx.guild_id]).await?.unwrap_or_else(|| PartialRoGuild::new(ctx.guild_id));
    if guild.kind.unwrap() == GuildType::Free {
        let message = "Mass Update commands are only available to Premium servers";
        ctx.respond(&bot).content(message).unwrap().await?;
    }
    let mut conn = bot.cache.get();
    let _: () = conn
        .publish(
            "update-role",
            serde_json::to_vec(&MassUpdateQueueArguments {
                guild_id: ctx.guild_id,
                role_id: Some(RoleId::new(args.role)),
            })
            .unwrap(),
        )
        .await
        .map_err(|err| CacheError::from(err))?;

    tokio::time::sleep(Duration::from_secs(30)).await;
    let server = bot.server(ctx.guild_id).await?;
    let members = bot.cache.guild_members(ctx.guild_id).await?;

    for member in members {
        if let Some(member) = bot.member(ctx.guild_id, member).await? {
            if member.roles.contains(&RoleId::new(args.role)) {
                if let Err(err) = update_member(bot, &server, &guild, member).await {
                    tracing::error!(err = ?err);
                }
            }
        }
    }

    bot.http
        .create_message(ctx.channel_id.0)
        .content("`update-role` is complete")
        .await?;
    Ok(())
}

async fn update_member(
    bot: &BotContext,
    server: &CachedGuild,
    guild: &PartialRoGuild,
    member: CachedMember,
) -> Result<(), RoError> {
    tracing::debug!(user_id = ?member.id);
    if server.owner_id == member.id {
        return Ok(());
    }

    // Check for a full bypass
    for bypass_role in &guild.bypass_roles {
        if bypass_role.kind == BypassRoleKind::All && member.roles.contains(&bypass_role.role_id) {
            return Ok(());
        }
    }

    let Some(user) = bot
        .database
        .query_opt::<RoUser>(
            "SELECT * FROM roblox_users WHERE user_id = $1",
            &[&member.id],
        )
        .await?
    else {
        return Ok(());
    };

    let mut all_roles = guild
        .rankbinds
        .iter()
        .flat_map(|r| r.discord_roles.clone())
        .collect::<HashSet<_>>();
    all_roles.extend(
        guild
            .groupbinds
            .iter()
            .flat_map(|g| g.discord_roles.clone()),
    );
    all_roles.extend(
        guild
            .custombinds
            .iter()
            .flat_map(|g| g.discord_roles.clone()),
    );
    all_roles.extend(
        guild
            .assetbinds
            .iter()
            .flat_map(|g| g.discord_roles.clone()),
    );
    all_roles.extend(&guild.unverified_roles);
    all_roles.extend(&guild.verified_roles);
    let all_roles = all_roles.into_iter().unique().collect::<Vec<_>>();

    let update_user = UpdateUser {
        http: &bot.http,
        roblox: &bot.roblox,
        member: &member,
        user: &user,
        server: &server,
        guild: &guild,
        all_roles: &all_roles,
    };
    let (added_roles, removed_roles, nickname) = match update_user.execute().await {
        Ok(u) => u,
        Err(err) => match err {
            UpdateUserError::DenyList((_, denylist)) => {
                tracing::debug!("user on a deny list. {:?}", denylist);
                match denylist.action_type {
                    DenyListActionType::None => {}
                    DenyListActionType::Kick => {
                        tracing::debug!("kicking them");
                        let _ = bot
                            .http
                            .remove_guild_member(guild.guild_id.0, member.id.0)
                            .await;
                    }
                    DenyListActionType::Ban => {
                        tracing::debug!("banning them");
                        let _ = bot.http.create_ban(guild.guild_id.0, member.id.0).await;
                    }
                }
                return Ok(());
            }
            UpdateUserError::Generic(err) => return Err(err),
            _ => return Ok(()),
        },
    };

    let mut added_str = added_roles.iter().fold(String::new(), |mut s, a| {
        let _ = write!(s, "- <@&{}>\n", a.0);
        s
    });
    let mut removed_str = removed_roles.iter().fold(String::new(), |mut s, a| {
        let _ = write!(s, "- <@&{}>\n", a.0);
        s
    });
    if added_str.is_empty() {
        added_str = "None".into();
    }
    if removed_str.is_empty() {
        removed_str = "None".into();
    }

    if let Some(log_channel) = guild.log_channel {
        if !added_roles.is_empty()
            || !removed_roles.is_empty()
            || member.nickname.unwrap_or(member.username) != nickname
        {
            let log_embed = EmbedBuilder::new()
                .color(0x0034_98DB)
                .footer(EmbedFooterBuilder::new("RoWifi").build())
                .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
                .title("Mass Update")
                .description(format!("Update: <@{}>", member.id))
                .field(EmbedFieldBuilder::new("Nickname", nickname))
                .field(EmbedFieldBuilder::new("Added Roles", added_str))
                .field(EmbedFieldBuilder::new("Removed Roles", removed_str))
                .build();
            let _ = bot
                .http
                .create_message(log_channel.0)
                .embeds(&[log_embed])
                .await;
        }
    }

    Ok(())
}
