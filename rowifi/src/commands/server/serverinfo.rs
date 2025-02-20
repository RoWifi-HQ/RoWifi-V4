use itertools::Itertools;
use rowifi_framework::prelude::*;
use rowifi_models::discord::{
    http::interaction::{InteractionResponse, InteractionResponseType},
    util::Timestamp,
};
use twilight_mention::Mention;

pub async fn serverinfo(bot: Extension<BotContext>, command: Command<()>) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = serverinfo_func(&bot, &command.ctx).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

pub async fn serverinfo_func(bot: &BotContext, ctx: &CommandContext) -> CommandResult {
    let guild = bot
        .get_guild("SELECT * FROM guilds WHERE guild_id = $1", ctx.guild_id)
        .await?;
    let server = bot.server(ctx.guild_id).await?;

    let unverified_roles = if guild.unverified_roles.is_empty() {
        "None".to_string()
    } else {
        guild
            .unverified_roles
            .iter()
            .map(|r| r.0.mention().to_string())
            .join(" ")
    };
    let verified_roles = if guild.verified_roles.is_empty() {
        "None".to_string()
    } else {
        guild
            .verified_roles
            .iter()
            .map(|r| r.0.mention().to_string())
            .join(" ")
    };
    let settings = format!(
        "**Auto Detection**: {}\n**Sync XP on `/setrank`**: {}\n**Update On Join**: {}",
        guild.auto_detection.unwrap_or_default(),
        guild.sync_xp_on_setrank.unwrap_or_default(),
        guild.update_on_join.unwrap_or_default()
    );

    let embed = EmbedBuilder::new()
        .color(BLUE)
        .footer(EmbedFooterBuilder::new("RoWifi").build())
        .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
        .title(server.name)
        .field(EmbedFieldBuilder::new("Guild Id", ctx.guild_id.to_string()).inline())
        .field(
            EmbedFieldBuilder::new("Tier", format!("{}", guild.kind.unwrap_or_default())).inline(),
        )
        .field(EmbedFieldBuilder::new("Unverified Roles", unverified_roles).inline())
        .field(EmbedFieldBuilder::new("Verified Roles", verified_roles).inline())
        .field(EmbedFieldBuilder::new("Rankbinds", guild.rankbinds.len().to_string()).inline())
        .field(EmbedFieldBuilder::new("Groupbinds", guild.groupbinds.len().to_string()).inline())
        .field(EmbedFieldBuilder::new("Custombinds", guild.custombinds.len().to_string()).inline())
        .field(EmbedFieldBuilder::new("Assetbinds", guild.assetbinds.len().to_string()).inline())
        .field(EmbedFieldBuilder::new("XP Binds", guild.xp_binds.len().to_string()).inline())
        .field(
            EmbedFieldBuilder::new("Bypass Roles", guild.bypass_roles.len().to_string()).inline(),
        )
        .field(
            EmbedFieldBuilder::new(
                "Default Template",
                guild.default_template.unwrap_or_default().0,
            )
            .inline(),
        )
        .field(EmbedFieldBuilder::new("Settings", settings));
    ctx.respond(bot).embeds(&[embed.build()]).unwrap().await?;

    Ok(())
}
