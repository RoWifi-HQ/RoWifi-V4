use rowifi_core::assetbinds::add::{add_assetbind, AddAssetbindError, AssetbindArguments};
use rowifi_framework::prelude::*;
use rowifi_models::{
    bind::{AssetType, Template},
    discord::{
        http::interaction::{InteractionResponse, InteractionResponseType},
        util::Timestamp,
    },
    id::RoleId,
    roblox::id::AssetId,
};
use std::collections::HashMap;
use twilight_mention::Mention;

#[derive(Arguments, Debug)]
pub struct AssetbindRouteArguments {
    pub option: AssetType,
    pub asset_id: u64,
    pub template: String,
    pub priority: Option<i32>,
    pub discord_roles: Vec<RoleId>,
}

pub async fn new_assetbind(
    bot: Extension<BotContext>,
    command: Command<AssetbindRouteArguments>,
) -> impl IntoResponse {
    spawn_command(new_assetbind_func(bot, command.ctx, command.args));

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all, fields(args = ?args))]
pub async fn new_assetbind_func(
    bot: Extension<BotContext>,
    ctx: CommandContext,
    args: AssetbindRouteArguments,
) -> CommandResult {
    tracing::debug!("assetbinds new invoked");
    let guild = bot
        .get_guild(
            "SELECT guild_id, assetbinds FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;
    let server = bot.server(ctx.guild_id).await?;
    let server_roles = bot
        .cache
        .guild_roles(server.roles.iter().copied())
        .await?
        .into_iter()
        .map(|r| (r.id, r))
        .collect::<HashMap<_, _>>();

    let res = match add_assetbind(
        &bot.database,
        ctx.guild_id,
        ctx.author_id,
        &guild.assetbinds,
        &server_roles,
        AssetbindArguments {
            kind: args.option,
            asset_id: AssetId(args.asset_id),
            template: Template(args.template),
            discord_roles: args.discord_roles,
            priority: args.priority,
        },
    )
    .await
    {
        Ok(res) => res,
        Err(AddAssetbindError::Generic(err)) => return Err(err),
    };

    let mut description: String = String::new();
    if res.modified {
        description.push_str(":warning: Bind already exists. Modified it to:\n\n");
    }
    description.push_str(&format!("**Asset Id: {}**\n", res.bind.asset_id));
    description.push_str(&format!(
        "Type: {}\nTemplate: {}\nPriority: {}\n Roles: {}",
        res.bind.asset_type,
        res.bind.template,
        res.bind.priority,
        res.bind
            .discord_roles
            .into_iter()
            .map(|r| r.0.mention().to_string())
            .collect::<String>()
    ));

    if !res.ignored_roles.is_empty() {
        let ignored_roles_str = res
            .ignored_roles
            .iter()
            .map(|r| r.0.mention().to_string())
            .collect::<String>();
        description.push_str(&format!("\n\n🚫 Invalid Roles: {}", ignored_roles_str));
    }

    let embed = EmbedBuilder::new()
        .color(DARK_GREEN)
        .footer(EmbedFooterBuilder::new("RoWifi").build())
        .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
        .title("Bind Addition Successful")
        .description(description)
        .build();
    ctx.respond(&bot).embeds(&[embed]).unwrap().await?;

    Ok(())
}
