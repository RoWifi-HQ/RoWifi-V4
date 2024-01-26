use rowifi_core::denylists::add::{add_denylist, DenylistArguments};
use rowifi_framework::prelude::*;
use rowifi_models::{
    deny_list::{DenyListActionType, DenyListData, DenyListType},
    discord::{
        http::interaction::{InteractionResponse, InteractionResponseType},
        util::Timestamp,
    },
    roblox::id::GroupId,
};

#[derive(Arguments, Debug)]
pub struct DenylistRouteArguments {
    pub group_id: u64,
    pub action: DenyListActionType,
    pub reason: Option<String>,
}

pub async fn add_group_denylist(
    bot: Extension<BotContext>,
    command: Command<DenylistRouteArguments>,
) -> impl IntoResponse {
    spawn_command(add_group_denylist_func(bot, command.ctx, command.args));

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all, fields(args = ?args))]
pub async fn add_group_denylist_func(
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

    if bot
        .roblox
        .get_group(GroupId(args.group_id))
        .await
        .map_err(RoError::from)?
        .is_none()
    {
        let message = format!(
            r#"
Oh no! A group with the ID {} does not exist. Ensure you have entered the ID correctly and try again.
            "#,
            args.group_id
        );
        ctx.respond(&bot).content(&message).unwrap().await?;
        return Ok(());
    }

    let reason = args.reason.unwrap_or_else(|| "N/A".into());

    let denylist = add_denylist(
        &bot.database,
        ctx.guild_id,
        ctx.author_id,
        &guild.deny_lists.0,
        DenylistArguments {
            kind: DenyListType::Group,
            action: args.action,
            reason,
            data: DenyListData::Group(GroupId(args.group_id)),
        },
    )
    .await?;

    let name = format!("Type: {}", denylist.kind());
    let desc = format!("Group Id: {}\nReason: {}", args.group_id, denylist.reason);

    let embed = EmbedBuilder::new()
        .color(DARK_GREEN)
        .footer(EmbedFooterBuilder::new("RoWifi").build())
        .timestamp(Timestamp::from_secs(OffsetDateTime::now_utc().unix_timestamp()).unwrap())
        .title("Denylist Addition Successful")
        .field(EmbedFieldBuilder::new(name.clone(), desc.clone()))
        .build();
    ctx.respond(&bot).embeds(&[embed])?.await?;

    Ok(())
}
