use rowifi_core::custombinds::delete::{delete_custombinds, CustombindArguments};
use rowifi_framework::prelude::*;
use rowifi_models::discord::{
    http::interaction::{InteractionResponse, InteractionResponseType},
    util::Timestamp,
};

#[derive(Arguments, Debug)]
pub struct CustombindRouteArguments {
    pub id: u32,
}

pub async fn delete_custombind(
    bot: Extension<BotContext>,
    command: Command<CustombindRouteArguments>,
) -> impl IntoResponse {
    spawn_command(delete_custombind_func(bot, command.ctx, command.args));

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all, fields(args = ?args))]
pub async fn delete_custombind_func(
    bot: Extension<BotContext>,
    ctx: CommandContext,
    args: CustombindRouteArguments,
) -> CommandResult {
    let guild = bot
        .get_guild(
            "SELECT guild_id, custombinds FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;

    let res = delete_custombinds(
        &bot.database,
        &guild.custombinds,
        ctx.guild_id,
        ctx.author_id,
        vec![CustombindArguments {
            custom_bind_id: args.id,
        }],
    )
    .await?;

    if res.invalid.first().is_some() {
        let embed = EmbedBuilder::new()
            .color(RED)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
            .title("Deletion Failed")
            .description(format!(
                "Custombind with ID {} does not exist",
                args.id
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

    Ok(())
}
