use rowifi_core::custombinds::add::{add_custombind, CustombindArguments};
use rowifi_framework::prelude::*;
use rowifi_models::{
    bind::Template,
    discord::{
        http::interaction::{InteractionResponse, InteractionResponseType},
        util::Timestamp,
    },
    id::RoleId,
};
use std::collections::HashMap;
use twilight_mention::Mention;

#[derive(Arguments, Debug)]
pub struct CustombindRouteArguments {
    pub code: String,
    pub template: String,
    pub priority: Option<i32>,
    pub discord_roles: Vec<RoleId>,
}

pub async fn new_custombind(
    bot: Extension<BotContext>,
    command: Command<CustombindRouteArguments>,
) -> impl IntoResponse {
    spawn_command(new_custombind_func(bot, command.ctx, command.args));

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all, fields(args = ?args))]
async fn new_custombind_func(
    bot: Extension<BotContext>,
    ctx: CommandContext,
    args: CustombindRouteArguments,
) -> CommandResult {
    let guild = bot
        .get_guild(
            "SELECT guild_id, custombinds FROM guilds WHERE guild_id = $1",
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

    let res = add_custombind(
        &bot.database,
        ctx.guild_id,
        ctx.author_id,
        &guild.custombinds,
        &server_roles,
        CustombindArguments {
            code: args.code,
            template: Template(args.template),
            priority: args.priority,
            discord_roles: args.discord_roles,
        },
    )
    .await?;

    let mut description = String::new();
    description.push_str(&format!("**Bind Id: {}**\n", res.bind.custom_bind_id));
    description.push_str(&format!(
        "Code: {}\nTemplate: {}\nPriority: {}\n Roles: {}",
        res.bind.code,
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
        .timestamp(Timestamp::from_secs(OffsetDateTime::now_utc().unix_timestamp()).unwrap())
        .title("Bind Addition Successful")
        .description(description)
        .build();
    ctx.respond(&bot).embeds(&[embed]).unwrap().await?;

    Ok(())
}