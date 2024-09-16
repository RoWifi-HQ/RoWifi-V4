use rowifi_core::rankbinds::delete::{delete_rankbinds, RankbindArguments};
use rowifi_framework::prelude::*;
use rowifi_models::{
    discord::{
        http::interaction::{InteractionResponse, InteractionResponseType},
        util::Timestamp,
    },
    roblox::id::GroupId,
};

#[derive(Arguments, Debug)]
pub struct RankbindRouteArguments {
    pub group_id: u64,
    pub rank_id: u32,
}

pub async fn delete_rankbind(
    bot: Extension<BotContext>,
    command: Command<RankbindRouteArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = delete_rankbind_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all, fields(args = ?args))]
pub async fn delete_rankbind_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: RankbindRouteArguments,
) -> CommandResult {
    let guild = bot
        .get_guild(
            "SELECT guild_id, rankbinds FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;

    let res = delete_rankbinds(
        &bot.database,
        &guild.rankbinds,
        ctx.guild_id,
        ctx.author_id,
        vec![RankbindArguments {
            group_id: GroupId(args.group_id),
            rank_id: args.rank_id,
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
                "Rankbind with Group ID {} and Rank ID {} does not exist",
                args.group_id, args.rank_id
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
