use itertools::Itertools;
use rowifi_framework::prelude::*;
use rowifi_models::{
    deny_list::DenyListActionType, discord::util::Timestamp, guild::BypassRoleKind, id::UserId,
    user::RoUser,
};
use std::{error::Error, fmt::Write};
use twilight_http::error::{Error as DiscordHttpError, ErrorType as DiscordErrorType};

use crate::{
    commands::{CommandError, CommandErrorType},
    utils::update_user::{UpdateUser, UpdateUserError},
};

#[derive(Arguments, Debug)]
pub struct UpdateArguments {
    pub user_id: Option<UserId>,
}

#[tracing::instrument(skip(ctx))]
pub async fn update_func(ctx: CommandContext, args: UpdateArguments) -> Result<(), FrameworkError> {
    tracing::debug!("update command invoked");
    ctx.defer_response(DeferredResponse::Normal).await?;
    let server = ctx.bot.cache.guild(ctx.guild_id).await?.unwrap();

    let user_id = match args.user_id {
        Some(s) => s,
        None => ctx.author_id,
    };
    tracing::trace!("running on {}", user_id);

    let Some(member) = ctx.bot.member(server.id, user_id).await? else {
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
        return Err(CommandError::from((CommandErrorType::UserNotFound, message)).into());
    };

    if server.owner_id == member.id {
        tracing::debug!("update running on server owner. aborting...");
        let message = r"
        👋 Hey there Server Owner, Discord prevents bots from modifying a server owner's nickname. Hence, RoWifi does not allow running the `/update` command on server owners.
        ";
        ctx.respond().content(message).unwrap().exec().await?;
        return Ok(());
    }

    let guild = ctx
        .bot
        .get_guild(
            "SELECT guild_id, bypass_roles, unverified_roles, verified_roles, rankbinds, groupbinds, assetbinds, deny_lists, default_template FROM guilds WHERE guild_id = $1",
            server.id,
        )
        .await?;
    tracing::trace!(guild = ?guild);

    // Check if the user has a bypass role for both (roles & nickname)
    for bypass_role in &guild.bypass_roles.0 {
        if bypass_role.kind == BypassRoleKind::All && member.roles.contains(&bypass_role.role_id) {
            tracing::debug!("detected bypass role({}). aborting...", bypass_role.role_id);
            let message = format!(
                r#"
<:rowifi:733311296732266577> **Update Bypass Detected**

You have a role (<@&{}>) which has been marked as a bypass role.
            "#,
                bypass_role.role_id
            );
            ctx.respond().content(&message).unwrap().exec().await?;
            return Ok(());
        }
    }

    let Some(user) = ctx
        .bot
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
Hey there, it looks like you're not verified with us. Please run `/verify` to register with us.
            "#
            )
        };
        ctx.respond().content(&message).unwrap().exec().await?;
        return Ok(());
    };
    tracing::trace!(user = ?user);

    let all_roles = guild
        .rankbinds
        .0
        .iter()
        .flat_map(|b| b.discord_roles.clone())
        .unique()
        .collect::<Vec<_>>();

    let update_user = UpdateUser {
        ctx: &ctx.bot,
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
                ctx.respond().content(&message).unwrap().exec().await?;

                match deny_list.action_type {
                    DenyListActionType::None => {}
                    DenyListActionType::Kick => {
                        tracing::trace!("kicking them");
                        let _ = ctx
                            .bot
                            .http
                            .remove_guild_member(ctx.guild_id.0, member.id.0)
                            .await;
                    }
                    DenyListActionType::Ban => {
                        tracing::trace!("banning them");
                        let _ = ctx.bot.http.create_ban(ctx.guild_id.0, member.id.0).await;
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
                ctx.respond().content(&message).unwrap().exec().await?;

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
                            ctx.respond().content(message).unwrap().exec().await?;
                            return Ok(());
                        }
                    }
                }
                return Err(err);
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
        .timestamp(Timestamp::from_secs(OffsetDateTime::now_utc().unix_timestamp()).unwrap())
        .title("Update")
        .field(EmbedFieldBuilder::new("Nickname", nickname))
        .field(EmbedFieldBuilder::new("Added Roles", added_str))
        .field(EmbedFieldBuilder::new("Removed Roles", removed_str))
        .build();
    ctx.respond().embeds(&[embed]).unwrap().exec().await?;

    Ok(())
}
