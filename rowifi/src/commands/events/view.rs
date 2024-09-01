use std::sync::Arc;

use itertools::Itertools;
use rowifi_framework::{prelude::*, utils::paginate_embeds};
use rowifi_models::{
    discord::{
        http::interaction::{InteractionResponse, InteractionResponseType},
        util::Timestamp,
    },
    events::EventLog,
    guild::GuildType,
    user::RoUser,
};
use twilight_standby::Standby;

#[derive(Arguments, Debug)]
pub struct EventViewArguments {
    pub username: Option<String>,
}

#[derive(Arguments, Debug)]
pub struct EventViewIdArguments {
    pub event_id: u64,
}

pub async fn view_attendee_events(
    bot: Extension<BotContext>,
    standby: Extension<Arc<Standby>>,
    command: Command<EventViewArguments>,
) -> impl IntoResponse {
    spawn_command(view_attendee_events_func(
        bot,
        standby,
        command.ctx,
        command.args,
    ));

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

pub async fn view_host_events(
    bot: Extension<BotContext>,
    standby: Extension<Arc<Standby>>,
    command: Command<EventViewArguments>,
) -> impl IntoResponse {
    spawn_command(view_host_events_func(
        bot,
        standby,
        command.ctx,
        command.args,
    ));

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

pub async fn view_event(
    bot: Extension<BotContext>,
    command: Command<EventViewIdArguments>,
) -> impl IntoResponse {
    spawn_command(view_event_func(bot, command.ctx, command.args));

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

pub async fn view_attendee_events_func(
    bot: Extension<BotContext>,
    standby: Extension<Arc<Standby>>,
    ctx: CommandContext,
    args: EventViewArguments,
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

    let roblox_id = match &args.username {
        Some(username) => {
            if let Some(u) = bot
                .roblox
                .get_users_from_usernames([username.as_str()].into_iter())
                .await?
                .into_iter()
                .next()
            {
                u.id
            } else {
                let message = format!(
                    "{} does not appear to be associated with any Roblox user.",
                    username
                );
                ctx.respond(&bot).content(&message)?.await?;
                return Ok(());
            }
        }
        None => {
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
            user.linked_accounts
                .get(&ctx.guild_id)
                .copied()
                .unwrap_or(user.default_account_id)
        }
    };

    let events = bot
        .database
        .query::<EventLog>(
            "SELECT * FROM events WHERE guild_id = $1 AND $2 = ANY(attendees)",
            &[&ctx.guild_id, &roblox_id],
        )
        .await?;

    let mut pages = Vec::new();
    let mut page_count = 0;

    for events in events.chunks(12) {
        let mut embed = EmbedBuilder::new()
            .color(DARK_GREEN)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
            .title("Attended Events")
            .description(format!("Page {}", page_count + 1));

        for event in events {
            let name = format!("Id: {}", event.guild_event_id);

            #[allow(clippy::cast_sign_loss)]
            let event_type = guild
                .event_types
                .iter()
                .find(|e| e.id == event.event_type as u32)
                .unwrap();
            let host = bot.roblox.get_user(event.host_id).await?;
            let desc = format!(
                "Event Type: {}\nHost: {}\nTimestamp: <t:{}:f>",
                event_type.name,
                host.name,
                event.timestamp.timestamp()
            );

            embed = embed.field(EmbedFieldBuilder::new(name, desc).inline());
        }
        pages.push(embed.build());
        page_count += 1;
    }

    paginate_embeds(&ctx, &bot, &standby, pages, page_count).await?;

    Ok(())
}

pub async fn view_host_events_func(
    bot: Extension<BotContext>,
    standby: Extension<Arc<Standby>>,
    ctx: CommandContext,
    args: EventViewArguments,
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

    let roblox_id = match &args.username {
        Some(username) => {
            if let Some(u) = bot
                .roblox
                .get_users_from_usernames([username.as_str()].into_iter())
                .await?
                .into_iter()
                .next()
            {
                u.id
            } else {
                let message = format!(
                    "{} does not appear to be associated with any Roblox user.",
                    username
                );
                ctx.respond(&bot).content(&message)?.await?;
                return Ok(());
            }
        }
        None => {
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
            user.linked_accounts
                .get(&ctx.guild_id)
                .copied()
                .unwrap_or(user.default_account_id)
        }
    };

    let events = bot
        .database
        .query::<EventLog>(
            "SELECT * FROM events WHERE guild_id = $1 AND host_id = $2",
            &[&ctx.guild_id, &roblox_id],
        )
        .await?;

    let mut pages = Vec::new();
    let mut page_count = 0;

    let host = bot.roblox.get_user(roblox_id).await?;
    for events in events.chunks(12) {
        let mut embed = EmbedBuilder::new()
            .color(DARK_GREEN)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
            .title("Hosted Events")
            .description(format!("Page {}", page_count + 1));

        for event in events {
            let name = format!("Id: {}", event.guild_event_id);

            #[allow(clippy::cast_sign_loss)]
            let event_type = guild
                .event_types
                .iter()
                .find(|e| e.id == event.event_type as u32)
                .unwrap();
            let desc = format!(
                "Event Type: {}\nHost: {}\nTimestamp: <t:{}:f>",
                event_type.name,
                host.name,
                event.timestamp.timestamp()
            );

            embed = embed.field(EmbedFieldBuilder::new(name, desc).inline());
        }
        pages.push(embed.build());
        page_count += 1;
    }

    paginate_embeds(&ctx, &bot, &standby, pages, page_count).await?;

    Ok(())
}

pub async fn view_event_func(
    bot: Extension<BotContext>,
    ctx: CommandContext,
    args: EventViewIdArguments,
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

    #[allow(clippy::cast_possible_wrap)]
    let Some(event) = bot
        .database
        .query_opt::<EventLog>(
            "SELECT * FROM events WHERE guild_id = $1 AND guild_event_id = $2",
            &[&ctx.guild_id, &(args.event_id as i64)],
        )
        .await?
    else {
        let message = format!("There is no event with the ID {}", args.event_id);
        ctx.respond(&bot).content(&message).unwrap().await?;
        return Ok(());
    };

    #[allow(clippy::cast_sign_loss)]
    let event_type = guild
        .event_types
        .iter()
        .find(|e| e.id == event.event_type as u32)
        .unwrap();
    let host = bot.roblox.get_user(event.host_id).await?;
    let mut attendees = Vec::new();
    for attendee in event.attendees {
        let user = bot.roblox.get_user(attendee).await?;
        attendees.push(user.name);
    }

    let mut embed = EmbedBuilder::new()
        .color(DARK_GREEN)
        .footer(EmbedFooterBuilder::new("RoWifi").build())
        .title(format!("Event Id: {}", event.guild_event_id))
        .field(EmbedFieldBuilder::new(
            "Event Type",
            event_type.name.clone(),
        ))
        .field(EmbedFieldBuilder::new("Host", host.name))
        .timestamp(Timestamp::from_secs(event.timestamp.timestamp()).unwrap());

    if attendees.is_empty() {
        embed = embed.field(EmbedFieldBuilder::new("Attendees", "None"));
    } else {
        embed = embed.field(EmbedFieldBuilder::new(
            "Attendees",
            attendees.iter().map(|a| format!("- {}", a)).join("\n"),
        ));
    }

    if let Some(notes) = event.notes {
        embed = embed.field(EmbedFieldBuilder::new("Notes", notes));
    }

    ctx.respond(&bot).embeds(&[embed.build()]).unwrap().await?;

    Ok(())
}
