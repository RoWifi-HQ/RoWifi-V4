use rowifi_database::Database;
use rowifi_models::id::UserId;

use crate::error::RoError;

pub struct BackupArguments {
    pub name: String,
}

pub async fn delete_backup(
    database: &Database,
    author: UserId,
    args: BackupArguments,
) -> Result<bool, RoError> {
    let rows = database
        .execute(
            "DELETE FROM backups WHERE user_id = $1 AND name = $2",
            &[&author, &args.name],
        )
        .await?;
    Ok(rows > 0)
}
