use rowifi_core::events::new::{log_event, EventLogArguments, EventLogError};
use rowifi_framework::prelude::*;
use rowifi_models::{
    discord::{
        http::interaction::{InteractionResponse, InteractionResponseType},
        util::Timestamp,
    },
    guild::GuildType,
    user::RoUser,
};

#[derive(Arguments, Debug)]
pub struct EventArguments {
    pub event_type: u32,
    pub attendees: String,
    pub notes: Option<String>,
}

pub async fn new_event(
    bot: Extension<BotContext>,
    command: Command<EventArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = new_event_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

pub async fn new_event_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: EventArguments,
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
        let message = "Event Logging is only available for Gamma Tier servers. You can upgrade the server on the dashboard.";
        ctx.respond(&bot).content(message).unwrap().await?;
        return Ok(());
    }

    let Some(user) = bot
        .database
        .query_opt::<RoUser>(
            "SELECT * FROM roblox_users WHERE user_id = $1",
            &[&ctx.author_id],
        )
        .await?
    else {
        let message = "Only verified users may log events.";
        ctx.respond(&bot).content(message).unwrap().await?;
        return Ok(());
    };

    let host_id = user
        .linked_accounts
        .get(&ctx.guild_id)
        .unwrap_or(&user.default_account_id);

    let attendee_ids = bot
        .roblox
        .get_users_from_usernames(args.attendees.split(|c| c == ' ' || c == ','))
        .await?
        .into_iter()
        .map(|u| u.id)
        .collect::<Vec<_>>();

    let new_event = match log_event(
        &bot.database,
        ctx.guild_id,
        ctx.author_id,
        &guild.event_types,
        EventLogArguments {
            host_id: *host_id,
            event_type: args.event_type,
            attendees: attendee_ids,
            notes: args.notes,
        },
    )
    .await
    {
        Ok(event) => event,
        Err(EventLogError::InvalidEventType) => {
            // Should not happen since the argument comes from slash commands
            let message = format!("There is no event type with the ID {}", args.event_type);
            ctx.respond(&bot).content(&message).unwrap().await?;
            return Ok(());
        }
        Err(EventLogError::Other(err)) => return Err(err),
    };

    let event_type = guild
        .event_types
        .iter()
        .find(|e| e.id == args.event_type)
        .unwrap();
    let value = format!(
        "Host: <@{}>\nType: {}\nAttendees: {}",
        ctx.author_id.get(),
        event_type.name,
        new_event.attendees.len()
    );
    let embed = EmbedBuilder::new()
        .color(DARK_GREEN)
        .footer(EmbedFooterBuilder::new("RoWifi").build())
        .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
        .title("Event Addition Successful")
        .field(EmbedFieldBuilder::new(
            format!("Event Id: {}", new_event.guild_event_id),
            value,
        ))
        .build();
    ctx.respond(&bot).embeds(&[embed]).unwrap().await?;

    Ok(())
}
