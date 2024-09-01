mod new;
mod restore;

pub use new::backup_new;
pub use restore::backup_restore;

use rowifi_framework::prelude::*;
use rowifi_models::discord::{
    http::interaction::{InteractionResponse, InteractionResponseType},
    util::Timestamp,
};

pub struct BackupRow {
    pub name: String,
    pub description: String,
}

pub async fn backup_view(bot: Extension<BotContext>, cmd: Command<()>) -> impl IntoResponse {
    spawn_command(backup_view_func(bot, cmd.ctx));

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

pub async fn backup_view_func(bot: Extension<BotContext>, ctx: CommandContext) -> CommandResult {
    let backups = bot
        .database
        .query::<BackupRow>(
            "SELECT name FROM backups WHERE user_id = $1",
            &[&ctx.author_id],
        )
        .await?;

    let mut embed = EmbedBuilder::new()
        .color(BLUE)
        .footer(EmbedFooterBuilder::new("RoWifi").build())
        .timestamp(Timestamp::from_secs(OffsetDateTime::now_utc().unix_timestamp()).unwrap())
        .title("Backups");

    for backup in backups {
        embed = embed.field(
            EmbedFieldBuilder::new(
                format!("Name: {}", backup.name),
                format!("Description: {}", backup.description),
            )
            .inline()
            .build(),
        );
    }

    ctx.respond(&bot).embeds(&[embed.build()]).unwrap().await?;
    Ok(())
}

impl TryFrom<rowifi_database::postgres::Row> for BackupRow {
    type Error = rowifi_database::postgres::Error;

    fn try_from(row: rowifi_database::postgres::Row) -> Result<Self, Self::Error> {
        let name = row.try_get("name")?;
        let description = row.try_get("description")?;

        Ok(Self { name, description })
    }
}