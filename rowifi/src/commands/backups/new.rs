use rowifi_core::backups::new::{create_backup, BackupArguments};
use rowifi_framework::prelude::*;
use rowifi_models::discord::http::interaction::{InteractionResponse, InteractionResponseType};

#[derive(Arguments, Debug)]
pub struct BackupRouteArguments {
    pub name: String,
}

pub async fn backup_new(
    bot: Extension<BotContext>,
    cmd: Command<BackupRouteArguments>,
) -> impl IntoResponse {
    spawn_command(backup_new_func(bot, cmd.ctx, cmd.args));

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

pub async fn backup_new_func(
    bot: Extension<BotContext>,
    ctx: CommandContext,
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

    ctx.respond(&bot)
        .content(&format!(
            "Backup {} has been created.",
            args.name
        ))
        .unwrap()
        .await?;

    Ok(())
}
