use rowifi_core::backups::new::{create_backup, BackupArguments};
use rowifi_framework::prelude::*;
use rowifi_models::discord::http::interaction::{InteractionResponse, InteractionResponseType};

#[derive(Arguments, Debug)]
pub struct BackupRouteArguments {
    pub name: String,
}

pub async fn backup_new(
    bot: Extension<BotContext>,
    command: Command<BackupRouteArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = backup_new_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

pub async fn backup_new_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: BackupRouteArguments,
) -> CommandResult {
    let guild = bot
        .get_guild("SELECT * FROM guilds WHERE guild_id = $1", ctx.guild_id)
        .await?;

    create_backup(
        &bot.database,
        &bot.cache,
        guild,
        BackupArguments {
            name: args.name.clone(),
            author: ctx.author_id,
        },
    )
    .await?;

    ctx.respond(bot)
        .content(&format!("Backup {} has been created.", args.name))
        .unwrap()
        .await?;

    Ok(())
}
