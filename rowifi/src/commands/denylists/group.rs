use rowifi_core::denylists::add::{add_denylist, AddDenylistError, DenylistArguments};
use rowifi_framework::prelude::*;
use rowifi_models::{
    deny_list::{DenyListActionType, DenyListType},
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
    tokio::spawn(async move {
        if let Err(err) = add_group_denylist_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all, fields(args = ?args))]
pub async fn add_group_denylist_func(
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

    if bot
        .roblox
        .get_group(GroupId(args.group_id))
        .await
        .map_err(RoError::from)?
        .is_none()
    {
        let message = format!(
            r"
Oh no! A group with the ID {} does not exist. Ensure you have entered the ID correctly and try again.
            ",
            args.group_id
        );
        ctx.respond(bot).content(&message).unwrap().await?;
        return Ok(());
    }

    let reason = args.reason.unwrap_or_else(|| "N/A".into());

    let denylist = match add_denylist(
        &bot.database,
        ctx.guild_id,
        ctx.author_id,
        guild.deny_lists,
        DenylistArguments {
            kind: DenyListType::Group,
            action: args.action,
            reason,
            user_id: None,
            group_id: Some(GroupId(args.group_id)),
            code: None,
        },
    )
    .await
    {
        Ok(res) => res,
        Err(
            AddDenylistError::MissingUser
            | AddDenylistError::MissingGroup
            | AddDenylistError::MissingCode
            | AddDenylistError::IncorrectCode(_),
        ) => {
            // Ignore this case since this won't occur in slash commands
            return Ok(());
        }
        Err(AddDenylistError::Generic(err)) => return Err(err),
    };

    let name = format!("Type: {}", denylist.kind());
    let desc = format!(
        "Group Id: {}\nAction: {}\nReason: {}",
        args.group_id, denylist.action_type, denylist.reason
    );

    let embed = EmbedBuilder::new()
        .color(DARK_GREEN)
        .footer(EmbedFooterBuilder::new("RoWifi").build())
        .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
        .title("Denylist Addition Successful")
        .field(EmbedFieldBuilder::new(&name, &desc))
        .build();
    ctx.respond(bot).embeds(&[embed])?.await?;

    if let Some(log_channel) = guild.log_channel {
        let embed = EmbedBuilder::new()
            .color(BLUE)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
            .title(format!("Action by <@{}>", ctx.author_id))
            .description("Denylist Added")
            .field(EmbedFieldBuilder::new(&name, &desc))
            .build();
        let _ = bot
            .http
            .create_message(log_channel.0)
            .embeds(&[embed])
            .await;
    }

    Ok(())
}
