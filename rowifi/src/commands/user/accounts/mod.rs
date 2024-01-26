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
pub use switch::account_switch;

pub async fn account_view(bot: Extension<BotContext>, command: Command<()>) -> impl IntoResponse {
    spawn_command(account_view_func(bot, command.ctx));

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all)]
pub async fn account_view_func(bot: Extension<BotContext>, ctx: CommandContext) -> CommandResult {
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

    let embed = EmbedBuilder::new()
        .color(BLUE)
        .footer(EmbedFooterBuilder::new("RoWifi").build())
        .timestamp(Timestamp::from_secs(OffsetDateTime::now_utc().unix_timestamp()).unwrap())
        .title("Linked Accounts");

    let mut acc_string = String::new();

    let main_user = bot.roblox.get_user(user.default_account_id).await?;
    acc_string.push_str(&main_user.display_name.unwrap_or(main_user.name));
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
        let alt_user = bot.roblox.get_user(account).await?;
        acc_string.push_str(&alt_user.name);
        if let Some(linked_user) = user.linked_accounts.get(&ctx.guild_id) {
            if *linked_user == account {
                acc_string.push_str(" - `This Server`");
            }
        }
        acc_string.push('\n');
    }

    let embed = embed.description(acc_string).build();
    ctx.respond(&bot).embeds(&[embed]).unwrap().await?;

    Ok(())
}
