mod delete;
mod group;
mod user;

use itertools::Itertools;
use rowifi_framework::{prelude::*, utils::paginate_embeds};
use rowifi_models::{
    deny_list::DenyListData,
    discord::{
        http::interaction::{InteractionResponse, InteractionResponseType},
        util::Timestamp,
    },
};
use std::sync::Arc;
use twilight_standby::Standby;

pub use delete::delete_denylist;
pub use group::add_group_denylist;
pub use user::add_user_denylist;

pub async fn view_denylists(
    bot: Extension<BotContext>,
    standby: Extension<Arc<Standby>>,
    command: Command<()>,
) -> impl IntoResponse {
    spawn_command(view_denylists_func(bot, standby, command.ctx));

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all)]
pub async fn view_denylists_func(
    bot: Extension<BotContext>,
    standby: Extension<Arc<Standby>>,
    ctx: CommandContext,
) -> CommandResult {
    let guild = bot
        .get_guild(
            "SELECT guild_id, deny_lists FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;

    if guild.deny_lists.is_empty() {
        let message = r"
This server has no denylists configured. Looking to add one? Use the command `/denylists new`.
        ";
        ctx.respond(&bot).content(message).unwrap().await?;
        return Ok(());
    }

    let mut pages = Vec::new();
    let mut page_count = 0usize;
    let denylists = guild.deny_lists;
    for denylist_chunk in &denylists.into_iter().chunks(12) {
        let mut embed = EmbedBuilder::new()
            .color(DARK_GREEN)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(OffsetDateTime::now_utc().unix_timestamp()).unwrap())
            .title("Denylists")
            .description(format!("Page {}", page_count + 1));
        for denylist in denylist_chunk {
            let name = format!("ID: {}", denylist.id);
            let mut desc = format!("Type: `{}`\nReason: {}\n", denylist.kind(), denylist.reason,);
            match denylist.data {
                DenyListData::User(user_id) => desc.push_str(&format!("User ID: {user_id}")),
                DenyListData::Group(group_id) => desc.push_str(&format!("Group ID: {group_id}")),
            }
            embed = embed.field(EmbedFieldBuilder::new(name, desc).inline().build());
        }
        pages.push(embed.build());
        page_count += 1;
    }

    paginate_embeds(&ctx, &bot, &standby, pages, page_count).await?;

    Ok(())
}
