use deadpool_redis::redis::AsyncCommands;
use rowifi_cache::error::CacheError;
use rowifi_framework::prelude::*;
use rowifi_models::{
    discord::http::interaction::{
        InteractionResponse, InteractionResponseData, InteractionResponseType,
    },
    id::GuildId,
};
use serde::Serialize;

pub async fn update_all(bot: Extension<BotContext>, command: Command<()>) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = update_all_func(&bot, &command.ctx).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some("`update-all` queue started".into()),
            ..Default::default()
        }),
    })
}

pub async fn update_all_func(bot: &BotContext, ctx: &CommandContext) -> CommandResult {
    let mut conn = bot.cache.get().await.map_err(|err| CacheError::from(err))?;
    let _: () = conn
        .publish("update-all", &ctx.guild_id.get())
        .await
        .map_err(|err| CacheError::from(err))?;
    Ok(())
}

#[derive(Arguments, Debug)]
pub struct UpdateRoleArguments {
    // TODO: Change this to RoleId
    pub role: u64,
}

#[derive(Debug, Serialize)]
pub struct UpdateRoleQueueArguments {
    pub guild: GuildId,
    pub role: u64,
}

pub async fn update_role(
    bot: Extension<BotContext>,
    command: Command<UpdateRoleArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = update_role_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some("`update-role` queue started".into()),
            ..Default::default()
        }),
    })
}

pub async fn update_role_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: UpdateRoleArguments,
) -> CommandResult {
    let mut conn = bot.cache.get().await.map_err(|err| CacheError::from(err))?;
    let _: () = conn
        .publish(
            "update-role",
            serde_json::to_vec(&UpdateRoleQueueArguments {
                guild: ctx.guild_id,
                role: args.role,
            })
            .unwrap(),
        )
        .await
        .map_err(|err| CacheError::from(err))?;
    Ok(())
}
