use rowifi_framework::prelude::*;
use rowifi_models::{id::UserId, guild::PartialRoGuild};

use crate::commands::{CommandError, CommandErrorType};

#[derive(Debug)]
pub struct UpdateArguments {
    pub user: Option<UserId>,
}

pub async fn update_func(
    ctx: &CommandContext,
    args: UpdateArguments,
) -> Result<(), FrameworkError> {
    let server = ctx.bot.cache.guild(ctx.guild_id).await?.unwrap();

    let user_id = match args.user {
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
            <:rwr:1183552637724012675> **Oh no!**

            Looks like there is no member with the id {}. 
            "#,
                user_id
            );
            return Err(CommandError::from((CommandErrorType::UserNotFound, message)).into());
        }
    };

    if server.owner_id == member.id {
        let message = 
            r#"
        <:rwr:1183552637724012675> Hey there Mr. Server Owner, Discord prevents bots from modifying
        a server owner's nickname. Hence, RoWifi does not update server owners.
        "#.into();
        return Err(CommandError::from((CommandErrorType::NoServerOwner, message)).into());
    }

    let guild = ctx.bot.get_guild("SELECT guild_id, bypass_roles FROM guilds WHERE guild_id = $1", server.id).await?;

    Ok(())
}
