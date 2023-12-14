use rowifi_framework::prelude::*;
use rowifi_models::id::UserId;

use crate::commands::{CommandError, CommandErrorType};

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

    if server.owner_id == member.id {
        let message = r#"
        👋 Hey there Mr. Server Owner, Discord prevents bots from modifying a server owner's nickname. Hence, RoWifi does not allow running the `/update` command on server owners.
        "#;
        ctx.respond().content(&message).unwrap().exec().await?;
        return Ok(());
    }

    let guild = ctx
        .bot
        .get_guild(
            "SELECT guild_id, bypass_roles, rankbinds FROM guilds WHERE guild_id = $1",
            server.id,
        )
        .await?;

    // Check if the user has a bypass role
    for bypass_role in guild.bypass_roles {
        if member.roles.contains(&bypass_role) {
            let message = format!(
                r#"
<:rowifi:733311296732266577> **Update Bypass Detected**

You have a role (<@&{bypass_role}>) which has been marked as a bypass role.
            "#
            );
            ctx.respond().content(&message).unwrap().exec().await?;
            return Ok(());
        }
    }

    Ok(())
}
