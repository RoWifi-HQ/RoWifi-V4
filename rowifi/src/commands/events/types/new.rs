use rowifi_core::events::types::add::{add_event_type, AddEventTypeError, EventTypeArguments};
use rowifi_framework::prelude::*;
use rowifi_models::{
    discord::{
        http::interaction::{InteractionResponse, InteractionResponseType},
        util::Timestamp,
    },
    guild::GuildType,
};

#[derive(Arguments, Debug)]
pub struct AddEventTypeArguments {
    pub id: u32,
    pub name: String,
}

pub async fn new_event_type(
    bot: Extension<BotContext>,
    command: Command<AddEventTypeArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = new_event_type_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

pub async fn new_event_type_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: AddEventTypeArguments,
) -> CommandResult {
    let guild = bot
        .get_guild(
            "SELECT guild_id, kind, event_types FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;

    // Check for Gamma Tier
    let kind = guild.kind.unwrap_or_default();
    if kind != GuildType::Gamma {
        let message = "The Events module is only available for Gamma Tier servers. You can upgrade the server on the dashboard.";
        ctx.respond(&bot).content(message).unwrap().await?;
        return Ok(());
    }

    let res = match add_event_type(
        &bot.database,
        ctx.guild_id,
        ctx.author_id,
        guild.event_types,
        EventTypeArguments {
            id: args.id,
            name: args.name,
        },
    )
    .await
    {
        Ok(e) => e,
        Err(AddEventTypeError::IdAlreadyExists) => {
            let message = format!(
                r#"
    An Event Type with ID {} already exists.
            "#,
                args.id
            );
            ctx.respond(&bot).content(&message).unwrap().await?;
            return Ok(());
        }
        Err(AddEventTypeError::Generic(err)) => return Err(err),
    };

    let embed = EmbedBuilder::new()
        .color(DARK_GREEN)
        .footer(EmbedFooterBuilder::new("RoWifi").build())
        .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
        .title("Event Type Creation Successful")
        .build();
    ctx.respond(&bot).embeds(&[embed]).unwrap().await?;

    if let Some(log_channel) = guild.log_channel {
        let embed = EmbedBuilder::new()
            .color(BLUE)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
            .title(format!("Action by <@{}>", ctx.author_id))
            .description("Event Type added")
            .field(EmbedFieldBuilder::new(
                format!("**Id: {}**\n", res.event.id),
                format!("Name: {}", res.event.name),
            ))
            .build();
        let _ = bot
            .http
            .create_message(log_channel.0)
            .embeds(&[embed])
            .await;
    }

    Ok(())
}
