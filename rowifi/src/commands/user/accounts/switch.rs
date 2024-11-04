use rowifi_framework::prelude::*;
use rowifi_models::{
    discord::http::interaction::{InteractionResponse, InteractionResponseType},
    user::RoUser,
};
use std::collections::HashMap;

#[derive(Arguments, Debug)]
pub struct AccountRouteArguments {
    pub username: String,
}

pub async fn account_switch(
    bot: Extension<BotContext>,
    command: Command<AccountRouteArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = account_switch_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all, fields(args = ?args))]
pub async fn account_switch_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: AccountRouteArguments,
) -> CommandResult {
    let Some(user) = bot
        .database
        .query_opt::<RoUser>(
            "SELECT * FROM roblox_users WHERE user_id = $1",
            &[&ctx.author_id],
        )
        .await?
    else {
        tracing::debug!("user is not in the database");
        let message = r"
Hey there, it looks like you're not verified with us. Please run `/verify` to register with RoWifi.
        ";
        ctx.respond(&bot).content(message).unwrap().await?;
        return Ok(());
    };

    let Some(roblox_user) = bot
        .roblox
        .get_users_from_usernames([args.username.as_str()].into_iter())
        .await?
        .into_iter()
        .next()
    else {
        let message = format!(
            r#"
Oh no! An account with the name `{}` does not seem to exist. Ensure you have spelled the username correctly and try again.
        "#,
            args.username
        );
        ctx.respond(&bot).content(&message).unwrap().await?;
        return Ok(());
    };

    if !user.other_accounts.contains(&roblox_user.id) && user.default_account_id != roblox_user.id {
        let message = format!(
            r#"
`{}` is not linked to your discord account. Link it using `/verify`. 
        "#,
            roblox_user.name
        );
        ctx.respond(&bot).content(&message).unwrap().await?;
        return Ok(());
    }

    let guild = bot.server(ctx.guild_id).await?;

    let mut map = HashMap::new();
    map.insert(ctx.guild_id.to_string(), Some(roblox_user.id.0.to_string()));
    bot.database
        .execute(
            "UPDATE roblox_users SET linked_accounts = linked_accounts || $2 WHERE user_id = $1",
            &[&ctx.author_id, &map],
        )
        .await?;
    bot.database.execute(
        "INSERT INTO linked_users(roblox_id, user_id, guild_id) VALUES($1, $2, $3) ON CONFLICT(guild_id, user_id) SET roblox_id = $1", 
        &[&roblox_user.id, &user.user_id, &ctx.guild_id]
    ).await?;

    let message = format!(
        r#"
Your account for **{}** was successfully set to **{}**.
    "#,
        guild.name, roblox_user.name
    );
    ctx.respond(&bot).content(&message).unwrap().await?;

    Ok(())
}
