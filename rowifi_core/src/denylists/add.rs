use rowifi_database::{postgres::types::Json, Database};
use rowifi_models::{
    audit_log::{AuditLog, AuditLogData, AuditLogKind},
    deny_list::{DenyList, DenyListActionType, DenyListData, DenyListType},
    id::{GuildId, UserId},
    roblox::id::{GroupId, UserId as RobloxUserId},
};
use serde::Deserialize;
use time::OffsetDateTime;

use crate::error::RoError;

#[derive(Debug, Deserialize)]
pub struct DenylistArguments {
    pub kind: DenyListType,
    pub action: DenyListActionType,
    pub reason: String,
    pub user_id: Option<RobloxUserId>,
    pub group_id: Option<GroupId>,
}

#[derive(Debug)]
pub enum AddDenylistError {
    MissingUser,
    MissingGroup,
    Generic(RoError),
}

/// Adds a denylist to the server. Modifies it if the denylist already exists.
///
/// # Errors
///
/// See [`AddDenylistError`] for details.
pub async fn add_denylist(
    database: &Database,
    guild_id: GuildId,
    author_id: UserId,
    existing_denylists: &[DenyList],
    args: DenylistArguments,
) -> Result<DenyList, AddDenylistError> {
    let data = if args.kind == DenyListType::User {
        if let Some(user_id) = args.user_id {
            DenyListData::User(user_id)
        } else {
            return Err(AddDenylistError::MissingUser);
        }
    } else if args.kind == DenyListType::Group {
        if let Some(group_id) = args.group_id {
            DenyListData::Group(group_id)
        } else {
            return Err(AddDenylistError::MissingGroup);
        }
    } else {
        unreachable!()
    };

    let denylist_id = existing_denylists.iter().map(|d| d.id).max().unwrap_or(0) + 1;
    let denylist = Json(DenyList {
        id: denylist_id,
        reason: args.reason,
        action_type: args.action,
        data,
    });

    let idx = existing_denylists
        .iter()
        .position(|d| d.data == denylist.0.data);

    database
        .execute(
            &format!(
                "UPDATE guilds SET deny_lists[{}] = $2 WHERE guild_id = $1",
                idx.unwrap_or(existing_denylists.len())
            ),
            &[&guild_id, &denylist],
        )
        .await
        .map_err(|err| AddDenylistError::Generic(err.into()))?;

    let log = AuditLog {
        kind: AuditLogKind::DenylistCreate,
        guild_id: Some(guild_id),
        user_id: Some(author_id),
        timestamp: OffsetDateTime::now_utc(),
        metadata: AuditLogData::DenylistCreate { kind: args.kind },
    };

    database
        .execute(
            r"INSERT INTO audit_logs(kind, guild_id, user_id, timestamp, metadata) 
        VALUES($1, $2, $3, $4, $5)",
            &[
                &log.kind,
                &log.guild_id,
                &log.user_id,
                &log.timestamp,
                &Json(log.metadata),
            ],
        )
        .await
        .map_err(|err| AddDenylistError::Generic(err.into()))?;

    Ok(denylist.0)
}
