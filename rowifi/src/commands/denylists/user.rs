use rowifi_core::denylists::add::{add_denylist, DenylistArguments};
use rowifi_framework::prelude::*;
use rowifi_models::{
    deny_list::{DenyListActionType, DenyListData, DenyListType},
    discord::{
        http::interaction::{InteractionResponse, InteractionResponseType},
        util::Timestamp,
    },
};

#[derive(Arguments, Debug)]
pub struct DenylistRouteArguments {
    pub username: String,
    pub action: DenyListActionType,
    pub reason: Option<String>,
}

pub async fn add_user_denylist(
    bot: Extension<BotContext>,
    command: Command<DenylistRouteArguments>,
) -> impl IntoResponse {
    spawn_command(add_user_denylist_func(bot, command.ctx, command.args));

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

pub async fn add_user_denylist_func(
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

    let Some(user) = bot.roblox.get_user_from_username(&args.username).await? else {
        let message = format!(
            r#"
Oh no! A user with the name `{}` does not exist.
        "#,
            args.username
        );
        ctx.respond(&bot).content(&message).unwrap().exec().await?;
        return Ok(());
    };

    let reason = args.reason.unwrap_or_else(|| "N/A".into());

    let denylist = add_denylist(
        &bot.database,
        ctx.guild_id,
        ctx.author_id,
        &guild.deny_lists.0,
        DenylistArguments {
            kind: DenyListType::User,
            action: args.action,
            reason,
            data: DenyListData::User(user.id),
        },
    )
    .await?;

    let name = format!("Type: {}", denylist.kind());
    let desc = format!("User Id: {}\nReason: {}", user.id, denylist.reason);

    let embed = EmbedBuilder::new()
        .color(DARK_GREEN)
        .footer(EmbedFooterBuilder::new("RoWifi").build())
        .timestamp(Timestamp::from_secs(OffsetDateTime::now_utc().unix_timestamp()).unwrap())
        .title("Blacklist Addition Successful")
        .field(EmbedFieldBuilder::new(name.clone(), desc.clone()))
        .build();
    ctx.respond(&bot).embeds(&[embed])?.exec().await?;

    Ok(())
}
