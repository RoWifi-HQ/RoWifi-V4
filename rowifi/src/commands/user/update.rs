use itertools::Itertools;
use rowifi_framework::prelude::*;
use rowifi_models::{id::UserId, user::RoUser, guild::BypassRoleKind};

use crate::{commands::{CommandError, CommandErrorType}, utils::update_user::{UpdateUser, UpdateUserError}};

#[derive(Arguments, Debug)]
pub struct UpdateArguments {
    pub user_id: Option<UserId>,
}

pub async fn update_func(ctx: CommandContext, args: UpdateArguments) -> Result<(), FrameworkError> {
    tracing::debug!("update command invoked");
    ctx.defer_response(DeferredResponse::Normal).await?;
    let server = ctx.bot.cache.guild(ctx.guild_id).await?.unwrap();

    let user_id = match args.user_id {
        Some(s) => s,
        None => ctx.author_id,
    };

    let member = match ctx.bot.member(server.id, user_id).await? {
        Some(m) => m,
        None => {
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
        }
    };

    // if server.owner_id == member.id {
    //     let message = r#"
    //     👋 Hey there Server Owner, Discord prevents bots from modifying a server owner's nickname. Hence, RoWifi does not allow running the `/update` command on server owners.
    //     "#;
    //     ctx.respond().content(&message).unwrap().exec().await?;
    //     return Ok(());
    // }

    let guild = ctx
        .bot
        .get_guild(
            "SELECT guild_id, bypass_roles, rankbinds FROM guilds WHERE guild_id = $1",
            server.id,
        )
        .await?;

    // Check if the user has a bypass role for both (roles & nickname)
    for bypass_role in &guild.bypass_roles.0 {
        if bypass_role.kind == BypassRoleKind::All && member.roles.contains(&bypass_role.role_id) {
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

    let user = match ctx
        .bot
        .database
        .query_opt::<RoUser>(
            "SELECT * FROM roblox_users WHERE user_id = $1",
            &[&member.id],
        )
        .await?
    {
        Some(u) => u,
        None => {
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
        }
    };

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
        all_roles: &all_roles
    };
    let (added_roles, removed_roles, nickname) = match update_user.execute().await {
        Ok(u) => u,
        Err(err) => match err {
            UpdateUserError::DenyList(_) => todo!(),
            UpdateUserError::InvalidNickname(_) => todo!(),
            UpdateUserError::Generic(err) => return Err(err) 
        }
    };

    Ok(())
}
