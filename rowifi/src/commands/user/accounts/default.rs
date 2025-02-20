use rowifi_framework::prelude::*;
use rowifi_models::{
    discord::http::interaction::{InteractionResponse, InteractionResponseType},
    user::RoUser,
};

#[derive(Arguments, Debug)]
pub struct AccountRouteArguments {
    pub username: String,
}

pub async fn account_default(
    bot: Extension<BotContext>,
    command: Command<AccountRouteArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = account_default_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all, fields(args = ?args))]
pub async fn account_default_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: AccountRouteArguments,
) -> CommandResult {
    let Some(mut user) = bot
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
        ctx.respond(bot).content(message).unwrap().await?;
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
            r"
Oh no! An account with the name `{}` does not seem to exist. Ensure you have spelled the username correctly and try again.
        ",
            args.username
        );
        ctx.respond(bot).content(&message).unwrap().await?;
        return Ok(());
    };

    if !user.other_accounts.contains(&roblox_user.id) && user.default_account_id != roblox_user.id {
        let message = format!(
            r"
`{}` is not linked to your discord account. Link it using `/verify`. 
        ",
            roblox_user.name
        );
        ctx.respond(bot).content(&message).unwrap().await?;
        return Ok(());
    }

    user.other_accounts.retain(|a| *a != roblox_user.id);
    user.other_accounts.push(user.default_account_id);

    bot.database
        .execute(
            "UPDATE roblox_users SET default_account_id = $2, other_accounts = $3 WHERE user_id = $1",
            &[&ctx.author_id, &roblox_user.id, &user.other_accounts],
        )
        .await?;

    let message = format!(
        r"
Your default account was successfully set to **{}**.
    ",
        roblox_user.name
    );
    ctx.respond(bot).content(&message).unwrap().await?;

    Ok(())
}
