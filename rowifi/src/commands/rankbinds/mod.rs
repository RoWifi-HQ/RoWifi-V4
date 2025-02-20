mod delete;
mod new;

use itertools::Itertools;
use rowifi_framework::{prelude::*, utils::paginate_embeds};
use rowifi_models::discord::{
    http::interaction::{InteractionResponse, InteractionResponseType},
    util::Timestamp,
};
use std::sync::Arc;
use twilight_mention::Mention;
use twilight_standby::Standby;

pub use delete::delete_rankbind;
pub use new::new_rankbind;

pub async fn view_rankbinds(
    bot: Extension<BotContext>,
    standby: Extension<Arc<Standby>>,
    command: Command<()>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = view_rankbinds_func(&bot, standby, &command.ctx).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all)]
pub async fn view_rankbinds_func(
    bot: &BotContext,
    standby: Extension<Arc<Standby>>,
    ctx: &CommandContext,
) -> CommandResult {
    let guild = bot
        .get_guild(
            "SELECT guild_id, rankbinds FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;

    if guild.rankbinds.is_empty() {
        let message = r"
This server has no rankbinds configured. Looking to add one? Use the command `/rankbinds new`.
        ";
        ctx.respond(bot).content(message).unwrap().await?;
        return Ok(());
    }

    let mut pages = Vec::new();
    let mut page_count = 0usize;
    let rankbinds = guild.rankbinds;
    for group in &rankbinds
        .into_iter()
        .sorted_by_key(|r| r.group_id)
        .chunk_by(|r| r.group_id)
    {
        for rbs in &group.1.chunks(12) {
            let mut embed = EmbedBuilder::new()
                .color(DARK_GREEN)
                .footer(EmbedFooterBuilder::new("RoWifi").build())
                .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
                .title("Rankbinds")
                .description(format!("Group {} | Page {}", group.0, page_count + 1));
            let rbs = rbs.sorted_unstable_by_key(|r| r.group_rank_id);
            for rb in rbs {
                let name = format!("Rank: {}", rb.group_rank_id);
                let desc = format!(
                    "Template: `{}`\nPriority: {}\n Roles: {}",
                    rb.template,
                    rb.priority,
                    rb.discord_roles
                        .iter()
                        .map(|r| r.0.mention().to_string())
                        .collect::<String>()
                );
                embed = embed.field(EmbedFieldBuilder::new(name, desc).inline().build());
            }
            pages.push(embed.build());
            page_count += 1;
        }
    }

    paginate_embeds(ctx, bot, &standby, pages, page_count).await?;

    Ok(())
}
