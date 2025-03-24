mod debug;

use futures_util::FutureExt;
use itertools::Itertools;
use rowifi_core::user::update::{UpdateUser, UpdateUserError};
use rowifi_framework::prelude::*;
use rowifi_models::{
    deny_list::{DenyList, DenyListActionType},
    discord::{
        cache::{CachedGuild, CachedMember},
        util::Timestamp,
    },
    guild::BypassRoleKind,
    id::UserId,
    user::RoUser,
};
use std::{error::Error, fmt::Write};
use twilight_http::error::{Error as DiscordHttpError, ErrorType as DiscordErrorType};

pub use debug::debug_update;

#[derive(Arguments, Debug)]
pub struct UpdateArguments {
    pub user_id: Option<UserId>,
}

pub async fn update_route(
    bot: Extension<BotContext>,
    command: Command<UpdateArguments>,
) -> impl IntoResponse {
    let _ = tokio::spawn(async move {
        if let Err(err) = update_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    })
    .await;
}

#[allow(clippy::too_many_lines)]
#[tracing::instrument(skip_all, fields(args = ?args))]
pub async fn update_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: UpdateArguments,
) -> CommandResult {
    ctx.defer_response(bot, DeferredResponse::Normal).await?;
    tracing::debug!("update command invoked");
    let server = bot.server(ctx.guild_id).await?;

    let user_id = match args.user_id {
        Some(s) => s,
        None => ctx.author_id,
    };

    let Some((discord_member, discord_user)) = bot.member(server.id, user_id).await? else {
        tracing::trace!("could not find user");
        // Should not ever happen since slash command guarantees that the user exists.
        // But handling this nonetheless is useful.
        let message = format!(
            r"
        <:rowifi:733311296732266577> **Oh no!**

        Looks like there is no member with the id {user_id}. 
        "
        );
        ctx.respond(bot).content(&message).unwrap().await?;
        return Ok(());
    };

    if server.owner_id == discord_member.id {
        tracing::debug!("update running on server owner. aborting...");
        let message = r"
        👋 Hey there Server Owner, Discord prevents bots from modifying a server owner's nickname. Hence, RoWifi does not allow running the `/update` command on server owners.
        ";
        ctx.respond(bot).content(message).unwrap().await?;
        return Ok(());
    }

    let guild = bot
        .get_guild(
            "SELECT guild_id, bypass_roles, unverified_roles, verified_roles, rankbinds, groupbinds, custombinds, assetbinds, deny_lists, default_template, sticky_roles, log_channel FROM guilds WHERE guild_id = $1",
            server.id,
        )
        .await?;
    tracing::trace!(guild = ?guild);

    // Check if the user has a bypass role for both (roles & nickname)
    for bypass_role in &guild.bypass_roles {
        if bypass_role.kind == BypassRoleKind::All
            && discord_member.roles.contains(&bypass_role.role_id)
        {
            tracing::debug!("detected bypass role({}). aborting...", bypass_role.role_id);
            let message = format!(
                r"
<:rowifi:733311296732266577> **Update Bypass Detected**

You have a role (<@&{}>) which has been marked as a bypass role.
            ",
                bypass_role.role_id
            );
            ctx.respond(bot).content(&message).unwrap().await?;
            return Ok(());
        }
    }

    let Some(user) = bot
        .database
        .query_opt::<RoUser>(
            "SELECT * FROM roblox_users WHERE user_id = $1",
            &[&discord_member.id],
        )
        .await?
    else {
        tracing::debug!("user is not in the database");
        let message = if args.user_id.is_some() {
            format!(
                r"
Oops, I did not find <@{}> in my database. They are not verified with RoWifi.
            ",
                discord_member.id
            )
        } else {
            r"
Hey there, it looks like you're not verified with us. Please run `/verify` to register with RoWifi.
            "
            .to_string()
        };
        ctx.respond(bot).content(&message).unwrap().await?;
        return Ok(());
    };
    tracing::trace!(user = ?user);

    let mut all_roles = guild
        .rankbinds
        .iter()
        .flat_map(|b| b.discord_roles.clone())
        .collect::<Vec<_>>();
    all_roles.extend(
        guild
            .groupbinds
            .iter()
            .flat_map(|b| b.discord_roles.clone()),
    );
    all_roles.extend(
        guild
            .custombinds
            .iter()
            .flat_map(|b| b.discord_roles.clone()),
    );
    all_roles.extend(
        guild
            .assetbinds
            .iter()
            .flat_map(|b| b.discord_roles.clone()),
    );
    all_roles.extend(&guild.unverified_roles);
    all_roles.extend(&guild.verified_roles);
    all_roles = all_roles.into_iter().unique().collect();

    let update_user = UpdateUser {
        http: &bot.http,
        roblox: &bot.roblox,
        discord_member: &discord_member,
        discord_user: &discord_user,
        user: &user,
        server: &server,
        guild: &guild,
        all_roles: &all_roles,
    };
    let (added_roles, removed_roles, nickname) = match update_user.execute().await {
        Ok(u) => u,
        Err(err) => match err {
            UpdateUserError::DenyList((_, deny_list)) => {
                tracing::debug!("user on a deny list. {:?}", deny_list);
                let message = if args.user_id.is_some() {
                    format!(
                        r"
    <@{}> is not allowed to be updated since they were found on a deny list. Reason: {}
                    ",
                        discord_member.id, deny_list.reason
                    )
                } else {
                    format!(
                        r"
    You are not allowed to run `/update` due to being on a deny list. Reason: {}
                    ",
                        deny_list.reason
                    )
                };
                ctx.respond(bot).content(&message).unwrap().await?;

                async fn dm_member(
                    bot: &BotContext,
                    server: &CachedGuild,
                    discord_member: &CachedMember,
                    deny_list: &DenyList,
                ) {
                    if let Ok(private_channel) =
                        bot.http.create_private_channel(discord_member.id.0).await
                    {
                        if let Ok(private_channel) = private_channel.model().await {
                            let _ = bot
                                .http
                                .create_message(private_channel.id)
                                .content(&format!(
                                    "You have been kicked from {}. Reason: {}",
                                    server.name, deny_list.reason
                                ))
                                .await;
                        }
                    }
                }

                match deny_list.action_type {
                    DenyListActionType::None => {}
                    DenyListActionType::Kick => {
                        tracing::trace!("kicking them");
                        dm_member(bot, &server, &discord_member, &deny_list)
                            .then(|()| async move {
                                let _ = bot
                                    .http
                                    .remove_guild_member(ctx.guild_id.0, discord_member.id.0)
                                    .await;
                            })
                            .await;
                    }
                    DenyListActionType::Ban => {
                        tracing::trace!("banning them");
                        dm_member(bot, &server, &discord_member, &deny_list)
                            .then(|()| async move {
                                let _ = bot
                                    .http
                                    .create_ban(ctx.guild_id.0, discord_member.id.0)
                                    .await;
                            })
                            .await;
                    }
                }

                return Ok(());
            }
            UpdateUserError::InvalidNickname(nickname) => {
                tracing::trace!("nickname({nickname}) more than 32 characters");
                let message = if args.user_id.is_some() {
                    if nickname.is_empty() {
                        format!(
                            r"
<@{}>'s supposed nickname is empty. Hence, they cannot be updated.
                        ",
                            discord_member.id
                        )
                    } else {
                        format!(
                            r"
<@{}>'s supposed nickname ({nickname}) is greater than 32 characters. Hence, they cannot be updated.
                    ",
                            discord_member.id
                        )
                    }
                } else if nickname.is_empty() {
                    r"
                Your supposed nickname is empty. Hence, you cannot be updated.
                                    "
                    .to_string()
                } else {
                    format!(
                        r"
                Your supposed nickname ({nickname}) is greater than 32 characters. Hence, you cannot be updated.
                                    ",
                    )
                };
                ctx.respond(bot).content(&message).unwrap().await?;

                return Ok(());
            }
            UpdateUserError::Generic(err) => {
                if let Some(source) = err
                    .source()
                    .and_then(|e| e.downcast_ref::<DiscordHttpError>())
                {
                    if let DiscordErrorType::Response {
                        body: _,
                        error: _,
                        status,
                    } = source.kind()
                    {
                        if *status == 403 {
                            let message = "There was an error in updating. Run `/debug update` to find potential issues";
                            ctx.respond(bot).content(message).unwrap().await?;
                            return Ok(());
                        }
                    }
                }
                return Err(err);
            }
            UpdateUserError::CustombindParsing { id, err } => {
                let message = format!("There was an error in parsing the custombind with ID {id}.");
                tracing::error!("{}", err);
                ctx.respond(bot).content(&message).unwrap().await?;
                return Ok(());
            }
            UpdateUserError::CustombindEvaluation { id, err } => {
                let message =
                    format!("There was an error in evaluating the custombind with ID {id}.");
                tracing::error!("{}", err);
                ctx.respond(bot).content(&message).unwrap().await?;
                return Ok(());
            }
            UpdateUserError::CustomDenylistEvaluation { id, err } => {
                let message =
                    format!("There was an error in evaluating the custom denylist with ID {id}.");
                tracing::error!("{}", err);
                ctx.respond(bot).content(&message).unwrap().await?;
                return Ok(());
            }
            UpdateUserError::CustomDenylistParsing { id, err } => {
                let message =
                    format!("There was an error in parsing the custom denylist with ID {id}.");
                tracing::error!("{}", err);
                ctx.respond(bot).content(&message).unwrap().await?;
                return Ok(());
            }
            UpdateUserError::BannedAccount(user_id) => {
                let message = format!(
                    "Your selected Roblox account for this server is {user_id}. It is believed to be a banned or suspected account. If this is not the case, please contact the RoWifi support server."
                );
                ctx.respond(bot).content(&message).unwrap().await?;
                return Ok(());
            }
        },
    };
    tracing::trace!(added_roles = ?added_roles, removed_roles = ?removed_roles, nickname = ?nickname);

    let mut added_str = added_roles.iter().fold(String::new(), |mut s, a| {
        let _ = writeln!(s, "- <@&{}>", a.0);
        s
    });
    let mut removed_str = removed_roles.iter().fold(String::new(), |mut s, a| {
        let _ = writeln!(s, "- <@&{}>", a.0);
        s
    });
    if added_str.is_empty() {
        added_str = "None".into();
    }
    if removed_str.is_empty() {
        removed_str = "None".into();
    }

    let embed = EmbedBuilder::new()
        .color(DARK_GREEN)
        .footer(EmbedFooterBuilder::new("RoWifi").build())
        .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
        .title("Update")
        .field(EmbedFieldBuilder::new("Nickname", &nickname))
        .field(EmbedFieldBuilder::new("Added Roles", &added_str))
        .field(EmbedFieldBuilder::new("Removed Roles", &removed_str))
        .build();
    ctx.respond(bot).embeds(&[embed]).unwrap().await?;

    if let Some(log_channel) = guild.log_channel {
        let log_embed = EmbedBuilder::new()
            .color(BLUE)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
            .title(format!("Action by <@{}>", ctx.author_id))
            .description(format!("Update: <@{}>", discord_member.id))
            .field(EmbedFieldBuilder::new("Nickname", &nickname))
            .field(EmbedFieldBuilder::new("Added Roles", &added_str))
            .field(EmbedFieldBuilder::new("Removed Roles", &removed_str))
            .build();
        let _ = bot
            .http
            .create_message(log_channel.0)
            .embeds(&[log_embed])
            .await;
    }

    Ok(())
}
