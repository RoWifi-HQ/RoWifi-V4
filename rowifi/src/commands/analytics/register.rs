use rowifi_framework::prelude::*;
use rowifi_models::{
    discord::{
        http::interaction::{InteractionResponse, InteractionResponseType},
        util::Timestamp,
    },
    guild::GuildType,
    roblox::id::GroupId,
};

#[derive(Arguments, Debug)]
pub struct RegisterArguments {
    pub group_id: GroupId,
}

pub async fn analytics_register(
    bot: Extension<BotContext>,
    command: Command<RegisterArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = analytics_register_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

pub async fn analytics_register_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: RegisterArguments,
) -> CommandResult {
    let mut guild = bot
        .get_guild(
            "SELECT guild_id, kind, registered_groups FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;

    if guild.kind.unwrap_or_default() != GuildType::Gamma {
        let message = "Analytics is only available for Gamma Tier servers. You can upgrade the server on the dashboard.";
        ctx.respond(bot).content(message).unwrap().await?;
        return Ok(());
    }

    if !guild.registered_groups.contains(&args.group_id) {
        guild.registered_groups.push(args.group_id);
        bot.database
            .execute(
                "UPDATE guilds SET registered_groups = $2 WHERE guild_id = $1",
                &[&ctx.guild_id, &guild.registered_groups],
            )
            .await?;
    }

    let embed = EmbedBuilder::new()
        .color(DARK_GREEN)
        .footer(EmbedFooterBuilder::new("RoWifi").build())
        .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
        .title("Group Added For Analytics")
        .build();
    ctx.respond(bot).embeds(&[embed]).unwrap().await?;

    Ok(())
}

pub async fn analytics_unregister(
    bot: Extension<BotContext>,
    command: Command<RegisterArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = analytics_unregister_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

pub async fn analytics_unregister_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: RegisterArguments,
) -> CommandResult {
    let mut guild = bot
        .get_guild(
            "SELECT guild_id, kind, registered_groups FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;

    if guild.kind.unwrap_or_default() != GuildType::Gamma {
        let message = "Analytics is only available for Gamma Tier servers. You can upgrade the server on the dashboard.";
        ctx.respond(bot).content(message).unwrap().await?;
        return Ok(());
    }

    let position = guild
        .registered_groups
        .iter()
        .position(|g| *g == args.group_id);
    if let Some(position) = position {
        guild.registered_groups.remove(position);
        bot.database
            .execute(
                "UPDATE guilds SET registered_groups = $2 WHERE guild_id = $1",
                &[&ctx.guild_id, &guild.registered_groups],
            )
            .await?;
    }

    let embed = EmbedBuilder::new()
        .color(DARK_GREEN)
        .footer(EmbedFooterBuilder::new("RoWifi").build())
        .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
        .title("Group Added For Analytics")
        .build();
    ctx.respond(bot).embeds(&[embed]).unwrap().await?;

    Ok(())
}
