use rowifi_core::custombinds::add::{add_custombind, AddCustombindError, CustombindArguments};
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
    tokio::spawn(async move {
        if let Err(err) = new_custombind_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all, fields(args = ?args))]
async fn new_custombind_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: CustombindRouteArguments,
) -> CommandResult {
    let guild = bot
        .get_guild(
            "SELECT guild_id, custombinds, log_channel FROM guilds WHERE guild_id = $1",
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

    let res = match add_custombind(
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
    .await
    {
        Ok(r) => r,
        Err(AddCustombindError::Code(err)) => {
            ctx.respond(&bot).content(&err).unwrap().await?;
            return Ok(());
        }
        Err(AddCustombindError::Other(err)) => return Err(err),
    };

    let mut description = String::new();
    description.push_str(&format!("**Bind Id: {}**\n", res.bind.custom_bind_id));
    description.push_str(&format!(
        "Code: {}\nTemplate: {}\nPriority: {}\n Roles: {}",
        res.bind.code,
        res.bind.template,
        res.bind.priority,
        res.bind
            .discord_roles
            .iter()
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

    if let Some(log_channel) = guild.log_channel {
        let embed = EmbedBuilder::new()
            .color(BLUE)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
            .title(format!("Action by <@{}>", ctx.author_id))
            .description("Custombind Added")
            .field(EmbedFieldBuilder::new(
                format!("**Bind Id: {}**\n", res.bind.custom_bind_id),
                format!(
                    "Code: {}\nTemplate: {}\nPriority: {}\n Roles: {}",
                    res.bind.code,
                    res.bind.template,
                    res.bind.priority,
                    res.bind
                        .discord_roles
                        .iter()
                        .map(|r| r.0.mention().to_string())
                        .collect::<String>()
                ),
            ))
            .build();
        let _ = bot
            .http
            .create_message(log_channel.0)
            .embeds(&[embed])
            .await;
    }

    Ok(())
}
