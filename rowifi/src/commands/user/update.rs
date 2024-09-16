use itertools::Itertools;
use rowifi_core::user::update::{UpdateUser, UpdateUserError};
use rowifi_framework::prelude::*;
use rowifi_models::{
    deny_list::DenyListActionType,
    discord::{
        http::interaction::{InteractionResponse, InteractionResponseType},
        util::Timestamp,
    },
    guild::BypassRoleKind,
    id::UserId,
    user::RoUser,
};
use std::{error::Error, fmt::Write};
use twilight_http::error::{Error as DiscordHttpError, ErrorType as DiscordErrorType};

#[derive(Arguments, Debug)]
pub struct UpdateArguments {
    pub user_id: Option<UserId>,
}

pub async fn update_route(
    bot: Extension<BotContext>,
    command: Command<UpdateArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = update_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[allow(clippy::too_many_lines)]
#[tracing::instrument(skip_all, fields(args = ?args))]
pub async fn update_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: UpdateArguments,
) -> CommandResult {
    tracing::debug!("update command invoked");
    let server = bot.server(ctx.guild_id).await?;

    let user_id = match args.user_id {
        Some(s) => s,
        None => ctx.author_id,
    };
    tracing::trace!("running on {}", user_id);

    let Some(member) = bot.member(server.id, user_id).await? else {
        tracing::trace!("could not find user");
        // Should not ever happen since slash command guarantees that the user exists.
        // But handling this nonetheless is useful.
        let message = format!(
            r#"
        <:rowifi:733311296732266577> **Oh no!**

        Looks like there is no member with the id {}. 
        "#,
            user_id
        );
        ctx.respond(&bot).content(&message).unwrap().await?;
        return Ok(());
    };

    if server.owner_id == member.id {
        tracing::debug!("update running on server owner. aborting...");
        let message = r"
        👋 Hey there Server Owner, Discord prevents bots from modifying a server owner's nickname. Hence, RoWifi does not allow running the `/update` command on server owners.
        ";
        ctx.respond(&bot).content(message).unwrap().await?;
        return Ok(());
    }

    let guild = bot
        .get_guild(
            "SELECT guild_id, bypass_roles, unverified_roles, verified_roles, rankbinds, groupbinds, custombinds, assetbinds, deny_lists, default_template FROM guilds WHERE guild_id = $1",
            server.id,
        )
        .await?;
    tracing::trace!(guild = ?guild);

    // Check if the user has a bypass role for both (roles & nickname)
    for bypass_role in &guild.bypass_roles {
        if bypass_role.kind == BypassRoleKind::All && member.roles.contains(&bypass_role.role_id) {
            tracing::debug!("detected bypass role({}). aborting...", bypass_role.role_id);
            let message = format!(
                r#"
<:rowifi:733311296732266577> **Update Bypass Detected**

You have a role (<@&{}>) which has been marked as a bypass role.
            "#,
                bypass_role.role_id
            );
            ctx.respond(&bot).content(&message).unwrap().await?;
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
        tracing::debug!("user is not in the database");
        let message = if args.user_id.is_some() {
            format!(
                r#"
Oops, I did not find <@{}> in my database. They are not verified with RoWifi.
            "#,
                member.id
            )
        } else {
            format!(
                r#"
Hey there, it looks like you're not verified with us. Please run `/verify` to register with RoWifi.
            "#
            )
        };
        ctx.respond(&bot).content(&message).unwrap().await?;
        return Ok(());
    };
    tracing::trace!(user = ?user);

    let all_roles = guild
        .rankbinds
        .iter()
        .flat_map(|b| b.discord_roles.clone())
        .unique()
        .collect::<Vec<_>>();

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
            UpdateUserError::DenyList(deny_list) => {
                tracing::debug!("user on a deny list. {:?}", deny_list);
                let message = if args.user_id.is_some() {
                    format!(
                        r#"
    <@{}> is not allowed to be updated since they were found on a deny list. Reason: {}
                    "#,
                        member.id, deny_list.reason
                    )
                } else {
                    format!(
                        r#"
    You are not allowed to run `/update` due to being on a deny list. Reason: {}
                    "#,
                        deny_list.reason
                    )
                };
                ctx.respond(&bot).content(&message).unwrap().await?;

                match deny_list.action_type {
                    DenyListActionType::None => {}
                    DenyListActionType::Kick => {
                        tracing::trace!("kicking them");
                        let _ = bot
                            .http
                            .remove_guild_member(ctx.guild_id.0, member.id.0)
                            .await;
                    }
                    DenyListActionType::Ban => {
                        tracing::trace!("banning them");
                        let _ = bot.http.create_ban(ctx.guild_id.0, member.id.0).await;
                    }
                }

                return Ok(());
            }
            UpdateUserError::InvalidNickname(nickname) => {
                tracing::debug!("nickname({nickname}) more than 32 characters");
                let message = if args.user_id.is_some() {
                    format!(
                        r#"
<@{}>'s supposed nickname ({nickname}) is greater than 32 characters. Hence, I cannot update them.
                    "#,
                        member.id
                    )
                } else {
                    format!(
                        r#"
Your supposed nickname ({nickname}) is greater than 32 characters. Hence, I cannot update you.
                    "#,
                    )
                };
                ctx.respond(&bot).content(&message).unwrap().await?;

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
                            ctx.respond(&bot).content(message).unwrap().await?;
                            return Ok(());
                        }
                    }
                }
                return Err(err);
            }
            UpdateUserError::CustombindParsing { id, err } => {
                let message = format!(
                    "There was an error in parsing the custombind with ID {}.\nError: `{}`",
                    id, err
                );
                ctx.respond(&bot).content(&message).unwrap().await?;
                return Ok(());
            }
            UpdateUserError::CustombindEvaluation { id, err } => {
                let message = format!(
                    "There was an error in evaluating the custombind with ID {}.\nError: `{}`",
                    id, err
                );
                ctx.respond(&bot).content(&message).unwrap().await?;
                return Ok(());
            }
        },
    };
    tracing::trace!(added_roles = ?added_roles, removed_roles = ?removed_roles, nickname = ?nickname);

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

    let embed = EmbedBuilder::new()
        .color(DARK_GREEN)
        .footer(EmbedFooterBuilder::new("RoWifi").build())
        .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
        .title("Update")
        .field(EmbedFieldBuilder::new("Nickname", nickname))
        .field(EmbedFieldBuilder::new("Added Roles", added_str))
        .field(EmbedFieldBuilder::new("Removed Roles", removed_str))
        .build();
    ctx.respond(&bot).embeds(&[embed]).unwrap().await?;

    Ok(())
}
