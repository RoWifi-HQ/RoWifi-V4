mod custom;
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

pub use custom::add_custom_denylist;
pub use delete::delete_denylist;
pub use group::add_group_denylist;
pub use user::add_user_denylist;

pub async fn view_denylists(
    bot: Extension<BotContext>,
    standby: Extension<Arc<Standby>>,
    command: Command<()>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = view_denylists_func(&bot, standby.0, &command.ctx).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all)]
pub async fn view_denylists_func(
    bot: &BotContext,
    standby: Arc<Standby>,
    ctx: &CommandContext,
) -> CommandResult {
    let guild = bot
        .get_guild(
            "SELECT guild_id, deny_lists FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;

    let mut pages = Vec::new();
    let mut page_count = 0usize;
    let denylists = guild.deny_lists;
    for denylist_chunk in &denylists.into_iter().chunks(12) {
        let mut embed = EmbedBuilder::new()
            .color(DARK_GREEN)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
            .title("Denylists")
            .description(format!("Page {}", page_count + 1));
        for denylist in denylist_chunk {
            let name = format!("ID: {}", denylist.id);
            let mut desc = format!(
                "Type: `{}`\nAction: {}\nReason: {}\n",
                denylist.kind(),
                denylist.action_type,
                denylist.reason
            );
            match denylist.data {
                DenyListData::User(user_id) => desc.push_str(&format!("User ID: {user_id}")),
                DenyListData::Group(group_id) => desc.push_str(&format!("Group ID: {group_id}")),
                DenyListData::Custom(code) => desc.push_str(&format!("Code: `{code}`")),
            }
            embed = embed.field(EmbedFieldBuilder::new(name, desc).inline().build());
        }
        pages.push(embed.build());
        page_count += 1;
    }

    paginate_embeds(ctx, bot, &standby, pages, page_count, "This server has no denylists configured. Looking to add one? Use the command `/denylists new`.").await?;

    Ok(())
}
