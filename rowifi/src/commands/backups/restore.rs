use rowifi_core::backups::restore::{restore_backup, BackupArguments, BackupError};
use rowifi_framework::prelude::*;
use rowifi_models::discord::http::interaction::{InteractionResponse, InteractionResponseType};

#[derive(Arguments, Debug)]
pub struct BackupRouteArguments {
    pub name: String,
}

pub async fn backup_restore(
    bot: Extension<BotContext>,
    cmd: Command<BackupRouteArguments>,
) -> impl IntoResponse {
    spawn_command(backup_restore_func(bot, cmd.ctx, cmd.args));

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

pub async fn backup_restore_func(
    bot: Extension<BotContext>,
    ctx: CommandContext,
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
