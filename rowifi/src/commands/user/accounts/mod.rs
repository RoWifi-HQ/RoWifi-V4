mod default;
mod delete;
mod switch;

use rowifi_framework::prelude::*;
use rowifi_models::{
    discord::{
        http::interaction::{InteractionResponse, InteractionResponseType},
        util::Timestamp,
    },
    user::RoUser,
};

pub use default::account_default;
pub use delete::account_delete;
use rowifi_roblox::error::ErrorKind;
pub use switch::account_switch;

pub async fn account_view(bot: Extension<BotContext>, command: Command<()>) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = account_view_func(&bot, &command.ctx).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all)]
pub async fn account_view_func(bot: &BotContext, ctx: &CommandContext) -> CommandResult {
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
        ctx.respond(bot).content(message).unwrap().await?;
        return Ok(());
    };

    let embed = EmbedBuilder::new()
        .color(BLUE)
        .footer(EmbedFooterBuilder::new("RoWifi").build())
        .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
        .title("Linked Accounts");

    let mut acc_string = String::new();

    let main_user = match bot.roblox.get_user(user.default_account_id).await {
        Ok(u) => Some(u),
        Err(err) => {
            if let ErrorKind::Response {
                route: _,
                status,
                bytes: _,
            } = err.kind()
            {
                if status.as_u16() == 404 {
                    None
                } else {
                    return Err(err.into());
                }
            } else {
                return Err(err.into());
            }
        }
    };
    let display_name = main_user.map_or_else(
        || user.default_account_id.to_string(),
        |m| m.display_name.unwrap_or(m.name),
    );
    acc_string.push_str(&display_name);
    acc_string.push_str(" - `Default`");

    // Check if there is an linked account for this server
    if let Some(linked_user) = user.linked_accounts.get(&ctx.guild_id) {
        if *linked_user == user.default_account_id {
            acc_string.push_str(", `This Server`");
        }
    } else {
        // There is no linked account for the server, so the default account is
        // linked one.
        acc_string.push_str(", `This Server`");
    }
    acc_string.push('\n');

    for account in user.other_accounts {
        let alt_user = match bot.roblox.get_user(user.default_account_id).await {
            Ok(u) => Some(u),
            Err(err) => {
                if let ErrorKind::Response {
                    route: _,
                    status,
                    bytes: _,
                } = err.kind()
                {
                    if status.as_u16() == 404 {
                        None
                    } else {
                        return Err(err.into());
                    }
                } else {
                    return Err(err.into());
                }
            }
        };
        let display_name = alt_user.map_or_else(
            || user.default_account_id.to_string(),
            |m| m.display_name.unwrap_or(m.name),
        );
        acc_string.push_str(&display_name);
        if let Some(linked_user) = user.linked_accounts.get(&ctx.guild_id) {
            if *linked_user == account {
                acc_string.push_str(" - `This Server`");
            }
        }
        acc_string.push('\n');
    }

    let embed = embed.description(acc_string).build();
    ctx.respond(bot).embeds(&[embed]).unwrap().await?;

    Ok(())
}
