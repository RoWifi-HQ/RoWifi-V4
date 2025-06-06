use rowifi_core::denylists::delete::delete_denylists;
use rowifi_framework::prelude::*;
use rowifi_models::discord::{
    http::interaction::{InteractionResponse, InteractionResponseType},
    util::Timestamp,
};

#[derive(Arguments, Debug)]
pub struct DenylistRouteArguments {
    pub id: u32,
}

pub async fn delete_denylist(
    bot: Extension<BotContext>,
    command: Command<DenylistRouteArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = delete_denylist_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all, fields(args = ?args))]
pub async fn delete_denylist_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: DenylistRouteArguments,
) -> CommandResult {
    let guild = bot
        .get_guild(
            "SELECT guild_id, deny_lists, log_channel FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;

    let res = delete_denylists(
        &bot.database,
        &guild.deny_lists,
        guild.guild_id,
        ctx.author_id,
        vec![args.id],
    )
    .await?;

    if res.invalid.is_empty() {
        let embed = EmbedBuilder::new()
            .color(DARK_GREEN)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
            .title("Deletion Successful")
            .build();
        ctx.respond(bot).embeds(&[embed]).unwrap().await?;
    } else {
        let embed = EmbedBuilder::new()
            .color(RED)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
            .title("Deletion Failed")
            .description(format!("Denylist with ID {} does not exist", args.id))
            .build();
        ctx.respond(bot).embeds(&[embed]).unwrap().await?;
    }

    if let Some(log_channel) = guild.log_channel {
        let embed = EmbedBuilder::new()
            .color(BLUE)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
            .title(format!("Action by <@{}>", ctx.author_id))
            .description(format!("Deleted {} denylist(s)", res.deleted))
            .build();
        let _ = bot
            .http
            .create_message(log_channel.0)
            .embeds(&[embed])
            .await;
    }

    Ok(())
}
