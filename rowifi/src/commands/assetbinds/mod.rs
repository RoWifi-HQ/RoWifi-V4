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

pub use delete::delete_assetbind;
pub use new::new_assetbind;

pub async fn view_assetbinds(
    bot: Extension<BotContext>,
    standby: Extension<Arc<Standby>>,
    command: Command<()>,
) -> impl IntoResponse {
    spawn_command(view_assetbinds_func(bot, standby, command.ctx));

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all)]
pub async fn view_assetbinds_func(
    bot: Extension<BotContext>,
    standby: Extension<Arc<Standby>>,
    ctx: CommandContext,
) -> CommandResult {
    let guild = bot
        .get_guild(
            "SELECT guild_id, assetbinds FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;

    if guild.assetbinds.is_empty() {
        let message = r"
This server has no assetbinds configured. Looking to add one? Use the command `/assetbinds new`.
        ";
        ctx.respond(&bot).content(message).unwrap().await?;
    }

    let mut pages = Vec::new();
    let mut page_count = 0usize;
    let assetbinds = guild.assetbinds;
    for abs in &assetbinds.into_iter().chunks(12) {
        let mut embed = EmbedBuilder::new()
            .color(DARK_GREEN)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
            .title("Assetbinds")
            .description(format!("Page {}", page_count + 1));
        for ab in abs {
            let name = format!("ID: {}", ab.asset_id);
            let desc = format!(
                "Type: `{}`\nTemplate: `{}`\nPriority: {}\n Roles: {}",
                ab.asset_type,
                ab.template,
                ab.priority,
                ab.discord_roles
                    .iter()
                    .map(|r| r.0.mention().to_string())
                    .collect::<String>()
            );
            embed = embed.field(EmbedFieldBuilder::new(name, desc).inline().build());
        }
        pages.push(embed.build());
        page_count += 1;
    }

    paginate_embeds(&ctx, &bot, &standby, pages, page_count).await?;

    Ok(())
}
