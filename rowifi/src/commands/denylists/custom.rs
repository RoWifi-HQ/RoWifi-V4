use rowifi_framework::prelude::*;
use rowifi_models::{
    deny_list::{DenyListActionType, DenyListType},
    discord::{
        http::interaction::{InteractionResponse, InteractionResponseType},
        util::Timestamp,
    },
};
use rowifi_core::denylists::add::{add_denylist, AddDenylistError, DenylistArguments};

#[derive(Arguments, Debug)]
pub struct DenylistRouteArguments {
    pub code: String,
    pub action: DenyListActionType,
    pub reason: Option<String>,
}

pub async fn add_custom_denylist(
    bot: Extension<BotContext>,
    command: Command<DenylistRouteArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = add_custom_denylist_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all, fields(args = ?args))]
pub async fn add_custom_denylist_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: DenylistRouteArguments,
) -> CommandResult {
    let guild = bot
        .get_guild(
            "SELECT guild_id, deny_lists FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;

    let reason = args.reason.unwrap_or_else(|| "N/A".into());

    let denylist = match add_denylist(
        &bot.database,
        ctx.guild_id,
        ctx.author_id,
        &guild.deny_lists,
        DenylistArguments {
            kind: DenyListType::Custom,
            action: args.action,
            reason,
            user_id: None,
            group_id: None,
            code: Some(args.code)
        },
    )
    .await
    {
        Ok(res) => res,
        Err(AddDenylistError::MissingUser | AddDenylistError::MissingGroup | AddDenylistError::MissingCode) => {
            // Ignore this case since it doesn't occur in slash commands
            return Ok(());
        },
        Err(AddDenylistError::IncorrectCode(err)) => {
            ctx.respond(&bot).content(&err).unwrap().await?;
            return Ok(());
        }
        Err(AddDenylistError::Generic(err)) => return Err(err),
    };

    let name = format!("Type: {}", denylist.kind());
    let desc = format!("User Id: {}\nAction: {}\nReason: {}", user.id, denylist.action_type, denylist.reason);

    let embed = EmbedBuilder::new()
        .color(DARK_GREEN)
        .footer(EmbedFooterBuilder::new("RoWifi").build())
        .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
        .title("Denylist Addition Successful")
        .field(EmbedFieldBuilder::new(name.clone(), desc.clone()))
        .build();
    ctx.respond(&bot).embeds(&[embed])?.await?;

    Ok(())
}