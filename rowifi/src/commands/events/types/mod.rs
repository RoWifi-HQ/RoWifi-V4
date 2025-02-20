mod new;

use itertools::Itertools;
use rowifi_framework::{prelude::*, utils::paginate_embeds};
use rowifi_models::{
    discord::{
        http::interaction::{InteractionResponse, InteractionResponseType},
        util::Timestamp,
    },
    guild::GuildType,
};
use std::sync::Arc;
use twilight_standby::Standby;

pub use new::new_event_type;

pub async fn view_event_types(
    bot: Extension<BotContext>,
    standby: Extension<Arc<Standby>>,
    command: Command<()>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = view_event_types_func(&bot, standby.0, &command.ctx).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

pub async fn view_event_types_func(
    bot: &BotContext,
    standby: Arc<Standby>,
    ctx: &CommandContext,
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
        ctx.respond(bot).content(message).unwrap().await?;
        return Ok(());
    }

    if guild.event_types.is_empty() {
        let message = r"
This server has no event types configured. Looking to add one? Use the command `/event-types new`.
        ";
        ctx.respond(bot).content(message).unwrap().await?;
        return Ok(());
    }

    let mut pages = Vec::new();
    let mut page_count = 0usize;
    let event_types = guild.event_types;
    for ets in &event_types.into_iter().chunks(12) {
        let mut embed = EmbedBuilder::new()
            .color(DARK_GREEN)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
            .title("Event Types")
            .description(format!("Page {}", page_count + 1));
        for et in ets {
            let name = format!("ID: {}", et.id);
            let desc = format!("Name: {}\n Disabled: {}", et.name, et.disabled);
            embed = embed.field(EmbedFieldBuilder::new(name, desc).inline().build());
        }
        pages.push(embed.build());
        page_count += 1;
    }

    paginate_embeds(ctx, bot, &standby, pages, page_count).await?;

    Ok(())
}
