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

pub async fn update_all(bot: Extension<BotContext>, cmd: Command<()>) -> impl IntoResponse {
    spawn_command(update_all_func(bot, cmd.ctx));

    Json(InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some("`update-all` queue started".into()),
            ..Default::default()
        }),
    })
}

pub async fn update_all_func(bot: Extension<BotContext>, ctx: CommandContext) -> CommandResult {
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
    cmd: Command<UpdateRoleArguments>,
) -> impl IntoResponse {
    spawn_command(update_role_func(bot, cmd.ctx, cmd.args));

    Json(InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some("`update-role` queue started".into()),
            ..Default::default()
        }),
    })
}

pub async fn update_role_func(
    bot: Extension<BotContext>,
    ctx: CommandContext,
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
