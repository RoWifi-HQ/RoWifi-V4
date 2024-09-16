use rowifi_core::backups::restore::{restore_backup, BackupArguments, BackupError};
use rowifi_framework::prelude::*;
use rowifi_models::discord::http::interaction::{InteractionResponse, InteractionResponseType};

#[derive(Arguments, Debug)]
pub struct BackupRouteArguments {
    pub name: String,
}

pub async fn backup_restore(
    bot: Extension<BotContext>,
    command: Command<BackupRouteArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = backup_restore_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

pub async fn backup_restore_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: BackupRouteArguments,
) -> CommandResult {
    match restore_backup(
        &bot.database,
        &bot.cache,
        &bot.http,
        ctx.author_id,
        BackupArguments {
            name: args.name.clone(),
        },
        ctx.guild_id,
    )
    .await
    {
        Ok(()) => {
            ctx.respond(&bot)
                .content(&format!(
                    "Backup {} has been restored to this server",
                    args.name
                ))
                .unwrap()
                .await?;
        }
        Err(BackupError::NotFound) => {
            ctx.respond(&bot)
                .content(&format!(
                    "There is no backup named {} linked to your account.",
                    args.name
                ))
                .unwrap()
                .await?;
        }
        Err(BackupError::Other(err)) => return Err(err),
    }

    Ok(())
}
