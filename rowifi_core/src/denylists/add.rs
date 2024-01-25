use rowifi_database::{postgres::types::Json, Database};
use rowifi_models::{
    audit_log::{AuditLog, AuditLogData, AuditLogKind},
    deny_list::{DenyList, DenyListActionType, DenyListData, DenyListType},
    id::{GuildId, UserId},
};
use time::OffsetDateTime;

use crate::error::RoError;

#[derive(Debug)]
pub struct DenylistArguments {
    pub kind: DenyListType,
    pub action: DenyListActionType,
    pub reason: String,
    pub data: DenyListData,
}

pub async fn add_denylist(
    database: &Database,
    guild_id: GuildId,
    author_id: UserId,
    existing_denylists: &[DenyList],
    args: DenylistArguments,
) -> Result<DenyList, RoError> {
    let denylist_id = existing_denylists.iter().map(|d| d.id).max().unwrap_or(0) + 1;
    let denylist = Json(DenyList {
        id: denylist_id,
        reason: args.reason,
        action_type: args.action,
        data: args.data,
    });

    let idx = existing_denylists
        .iter()
        .position(|d| d.data == denylist.0.data);

    database
        .execute(
            &format!(
                "UPDATE guilds SET deny_lists[{}] = $2 WHERE guild_id = $1",
                idx.unwrap_or_else(|| existing_denylists.len())
            ),
            &[&guild_id, &denylist],
        )
        .await
        .map_err(|err| RoError::from(err))?;

    let log = AuditLog {
        kind: AuditLogKind::DenylistCreate,
        guild_id: Some(guild_id),
        user_id: Some(author_id),
        timestamp: OffsetDateTime::now_utc(),
        metadata: AuditLogData::DenylistCreate { kind: args.kind },
    };

    database
        .execute(
            r#"INSERT INTO audit_logs(kind, guild_id, user_id, timestamp, metadata) 
        VALUES($1, $2, $3, $4, $5)"#,
            &[
                &log.kind,
                &log.guild_id,
                &log.user_id,
                &log.timestamp,
                &Json(log.metadata),
            ],
        )
        .await
        .map_err(|err| RoError::from(err))?;

    Ok(denylist.0)
}
