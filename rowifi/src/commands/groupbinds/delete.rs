use rowifi_core::groupbinds::delete::delete_groupbinds;
use rowifi_framework::prelude::*;
use rowifi_models::{
    discord::{
        http::interaction::{InteractionResponse, InteractionResponseType},
        util::Timestamp,
    },
    roblox::id::GroupId,
};

#[derive(Arguments, Debug)]
pub struct GroupbindRouteArguments {
    pub group_id: u64,
}

pub async fn delete_groupbind(
    bot: Extension<BotContext>,
    command: Command<GroupbindRouteArguments>,
) -> impl IntoResponse {
    spawn_command(delete_groupbind_func(bot, command.ctx, command.args));

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all, fields(args = ?args))]
pub async fn delete_groupbind_func(
    bot: Extension<BotContext>,
    ctx: CommandContext,
    args: GroupbindRouteArguments,
) -> CommandResult {
    let guild = bot
        .get_guild(
            "SELECT guild_id, groupbinds FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;

    let res = delete_groupbinds(
        &bot.database,
        &guild.groupbinds,
        ctx.guild_id,
        ctx.author_id,
        vec![GroupId(args.group_id)],
    )
    .await?;

    if res.invalid.first().is_some() {
        let embed = EmbedBuilder::new()
            .color(RED)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(OffsetDateTime::now_utc().unix_timestamp()).unwrap())
            .title("Deletion Failed")
            .description(format!(
                "Groupbind with ID {} does not exist",
                args.group_id
            ))
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
