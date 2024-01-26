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
    spawn_command(delete_denylist_func(bot, command.ctx, command.args));

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all, fields(args = ?args))]
pub async fn delete_denylist_func(
    bot: Extension<BotContext>,
    ctx: CommandContext,
    args: DenylistRouteArguments,
) -> CommandResult {
    let guild = bot
        .get_guild(
            "SELECT guild_id, deny_lists FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;

    let res = delete_denylists(
        &bot.database,
        &guild.deny_lists.0,
        guild.guild_id,
        ctx.author_id,
        vec![args.id],
    )
    .await?;

    if res.invalid.first().is_some() {
        let embed = EmbedBuilder::new()
            .color(RED)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(OffsetDateTime::now_utc().unix_timestamp()).unwrap())
            .title("Deletion Failed")
            .description(format!("Denylist with ID {} does not exist", args.id))
            .build();
        ctx.respond(&bot).embeds(&[embed]).unwrap().await?;
    } else {
        let embed = EmbedBuilder::new()
            .color(DARK_GREEN)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(OffsetDateTime::now_utc().unix_timestamp()).unwrap())
            .title("Deletion Successful")
            .build();
        ctx.respond(&bot).embeds(&[embed]).unwrap().await?;
    }

    Ok(())
}
