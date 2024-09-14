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

pub use delete::delete_groupbind;
pub use new::new_groupbind;

pub async fn view_groupbinds(
    bot: Extension<BotContext>,
    standby: Extension<Arc<Standby>>,
    command: Command<()>,
) -> impl IntoResponse {
    spawn_command(view_groupbinds_func(bot, standby, command.ctx));

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all)]
pub async fn view_groupbinds_func(
    bot: Extension<BotContext>,
    standby: Extension<Arc<Standby>>,
    ctx: CommandContext,
) -> CommandResult {
    let guild = bot
        .get_guild(
            "SELECT guild_id, groupbinds FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;

    if guild.groupbinds.is_empty() {
        let message = r"
This server has no groupbinds configured. Looking to add one? Use the command `/groupbinds new`.
        ";
        ctx.respond(&bot).content(message).unwrap().await?;
        return Ok(());
    }

    let mut pages = Vec::new();
    let mut page_count = 0usize;
    let groupbinds = guild.groupbinds;
    for gbs in &groupbinds.into_iter().chunks(12) {
        let mut embed = EmbedBuilder::new()
            .color(DARK_GREEN)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
            .title("Groupbinds")
            .description(format!("Page {}", page_count + 1));
        for gb in gbs {
            let name = format!("Group: {}", gb.group_id);
            let desc = format!(
                "Template: `{}`\nPriority: {}\n Roles: {}",
                gb.template,
                gb.priority,
                gb.discord_roles
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
