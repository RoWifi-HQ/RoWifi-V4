use rowifi_core::assetbinds::delete::delete_assetbinds;
use rowifi_framework::prelude::*;
use rowifi_models::{
    discord::{
        http::interaction::{InteractionResponse, InteractionResponseType},
        util::Timestamp,
    },
    roblox::id::AssetId,
};

#[derive(Arguments, Debug)]
pub struct AssetbindRouteArguments {
    pub asset_id: u64,
}

pub async fn delete_assetbind(
    bot: Extension<BotContext>,
    command: Command<AssetbindRouteArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = delete_assetbind_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all, fields(args = ?args))]
pub async fn delete_assetbind_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: AssetbindRouteArguments,
) -> CommandResult {
    let guild = bot
        .get_guild(
            "SELECT guild_id, assetbinds, log_channel FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;

    let res = delete_assetbinds(
        &bot.database,
        &guild.assetbinds,
        ctx.guild_id,
        ctx.author_id,
        vec![AssetId(args.asset_id)],
    )
    .await?;

    if res.invalid.first().is_some() {
        let embed = EmbedBuilder::new()
            .color(RED)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
            .title("Deletion Failed")
            .description(format!(
                "Assetbind with ID {} does not exist",
                args.asset_id
            ))
            .build();
        ctx.respond(&bot).embeds(&[embed]).unwrap().await?;
    } else {
        let embed = EmbedBuilder::new()
            .color(DARK_GREEN)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
            .title("Deletion Successful")
            .build();
        ctx.respond(&bot).embeds(&[embed]).unwrap().await?;
    }

    if let Some(log_channel) = guild.log_channel {
        let embed = EmbedBuilder::new()
            .color(BLUE)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
            .title(format!("Action by <@{}>", ctx.author_id))
            .description(format!("Deleted {} assetbind(s)", res.deleted))
            .build();
        let _ = bot
            .http
            .create_message(log_channel.0)
            .embeds(&[embed])
            .await;
    }

    Ok(())
}
