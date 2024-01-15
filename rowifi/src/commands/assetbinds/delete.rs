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
    spawn_command(delete_assetbind_func(bot, command.ctx, command.args));

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

pub async fn delete_assetbind_func(
    bot: Extension<BotContext>,
    ctx: CommandContext,
    args: AssetbindRouteArguments,
) -> CommandResult {
    let guild = bot
        .get_guild(
            "SELECT guild_id, assetbinds FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;

    let res = delete_assetbinds(
        &bot.database,
        &guild.assetbinds.0,
        ctx.guild_id,
        ctx.author_id,
        vec![AssetId(args.asset_id)],
    )
    .await?;

    if res.invalid.first().is_some() {
        let embed = EmbedBuilder::new()
            .color(RED)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(OffsetDateTime::now_utc().unix_timestamp()).unwrap())
            .title("Deletion Failed")
            .description(format!(
                "Assetbind with ID {} does not exist",
                args.asset_id
            ))
            .build();
        ctx.respond(&bot).embeds(&[embed]).unwrap().exec().await?;
    } else {
        let embed = EmbedBuilder::new()
            .color(DARK_GREEN)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(OffsetDateTime::now_utc().unix_timestamp()).unwrap())
            .title("Deletion Successful")
            .build();
        ctx.respond(&bot).embeds(&[embed]).unwrap().exec().await?;
    }

    Ok(())
}
