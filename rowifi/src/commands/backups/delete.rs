use rowifi_core::backups::delete::{delete_backup, BackupArguments};
use rowifi_framework::prelude::*;
use rowifi_models::discord::http::interaction::{InteractionResponse, InteractionResponseType};

#[derive(Arguments, Debug)]
pub struct BackupRouteArguments {
    pub name: String,
}

pub async fn backup_delete(
    bot: Extension<BotContext>,
    command: Command<BackupRouteArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = backup_delete_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

pub async fn backup_delete_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: BackupRouteArguments,
) -> CommandResult {
    let success = delete_backup(
        &bot.database,
        ctx.author_id,
        BackupArguments {
            name: args.name.clone(),
        },
    )
    .await?;

    if success {
        ctx.respond(&bot)
            .content(&format!("Backup {} has been deleted.", args.name))
            .unwrap()
            .await?;
    } else {
        ctx.respond(&bot)
            .content(&format!(
                "There is no backup named {} linked to your account.",
                args.name
            ))
            .unwrap()
            .await?;
    }

    Ok(())
}
