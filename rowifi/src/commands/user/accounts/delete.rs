use std::collections::HashMap;

use rowifi_framework::prelude::*;
use rowifi_models::{
    discord::http::interaction::{InteractionResponse, InteractionResponseType},
    user::RoUser,
};

#[derive(Arguments, Debug)]
pub struct AccountRouteArguments {
    pub username: String,
}

pub async fn account_delete(
    bot: Extension<BotContext>,
    command: Command<AccountRouteArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = account_delete_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all, fields(args = ?args))]
pub async fn account_delete_func(
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
`{}` is not linked to your discord account.
        "#,
            roblox_user.name
        );
        ctx.respond(&bot).content(&message).unwrap().await?;
        return Ok(());
    }

    if user.default_account_id == roblox_user.id {
        let message = r"
You cannot delete your default linked account.
        ";
        ctx.respond(&bot).content(message).unwrap().await?;
        return Ok(());
    }

    user.other_accounts.retain(|a| *a != roblox_user.id);
    user.linked_accounts.retain(|_, v| *v != roblox_user.id);

    let linked_accounts = user
        .linked_accounts
        .into_iter()
        .map(|(k, v)| (k.to_string(), Some(v.0.to_string())))
        .collect::<HashMap<_, _>>();
    bot.database
        .execute(
            "UPDATE roblox_users SET linked_accounts = $2, other_accounts = $3 WHERE user_id = $1",
            &[&ctx.author_id, &linked_accounts, &user.other_accounts],
        )
        .await?;

    let message = format!(
        r#"
**{}** was successfully unlinked from your Discord account.
    "#,
        roblox_user.name
    );
    ctx.respond(&bot).content(&message).unwrap().await?;

    Ok(())
}
