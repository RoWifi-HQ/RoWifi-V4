mod delete;
mod new;

use itertools::Itertools;
use rowifi_framework::{prelude::*, utils::paginate_embeds};
use std::sync::Arc;

pub use delete::delete_custombind;
pub use new::new_custombind;
use rowifi_models::discord::{
    http::interaction::{InteractionResponse, InteractionResponseType},
    util::Timestamp,
};
use twilight_mention::Mention;
use twilight_standby::Standby;

pub async fn view_custombinds(
    bot: Extension<BotContext>,
    standby: Extension<Arc<Standby>>,
    command: Command<()>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = view_custombinds_func(&bot, standby.0, &command.ctx).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all)]
pub async fn view_custombinds_func(
    bot: &BotContext,
    standby: Arc<Standby>,
    ctx: &CommandContext,
) -> CommandResult {
    let guild = bot
        .get_guild(
            "SELECT guild_id, custombinds FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;

    if guild.custombinds.is_empty() {
        let message = r"
This server has no custombinds configured. Looking to add one? Use the command `/custombinds new`.
        ";
        ctx.respond(bot).content(message).unwrap().await?;
    }

    let mut pages = Vec::new();
    let mut page_count = 0usize;
    let custombinds = guild.custombinds;
    for cbs in &custombinds.into_iter().chunks(12) {
        let mut embed = EmbedBuilder::new()
            .color(DARK_GREEN)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
            .title("Custombinds")
            .description(format!("Page {}", page_count + 1));
        for cb in cbs {
            let name = format!("Id: {}", cb.custom_bind_id);
            let desc = format!(
                "Code: {}\nTemplate: `{}`\nPriority: {}\n Roles: {}",
                cb.code,
                cb.template,
                cb.priority,
                cb.discord_roles
                    .iter()
                    .map(|r| r.0.mention().to_string())
                    .collect::<String>()
            );
            embed = embed.field(EmbedFieldBuilder::new(name, desc).inline().build());
        }
        pages.push(embed.build());
        page_count += 1;
    }

    paginate_embeds(ctx, bot, &standby, pages, page_count).await?;

    Ok(())
}
