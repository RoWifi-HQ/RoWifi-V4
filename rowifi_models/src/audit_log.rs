use bytes::BytesMut;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use tokio_postgres::types::{to_sql_checked, FromSql, IsNull, Json, ToSql, Type};

use crate::{
    bind::BindType,
    deny_list::DenyListType,
    id::{GuildId, UserId},
    roblox::id::{GroupId, UserId as RobloxUserId},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AuditLog {
    pub kind: AuditLogKind,
    // TODO: Write custom deserializer for this
    pub metadata: AuditLogData,
    pub guild_id: Option<GuildId>,
    pub user_id: Option<UserId>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Deserialize_repr, Eq, Ord, PartialEq, PartialOrd, Serialize_repr)]
#[repr(u16)]
pub enum AuditLogKind {
    BindCreate = 1,
    BindModify = 2,
    BindDelete = 3,
    XPAdd = 4,
    XPRemove = 5,
    SetRank = 6,
    XPSet = 7,
    DenylistCreate = 8,
    DenylistDelete = 9,
    EventLog = 10,
    SettingModify = 11,
    EventTypeCreate = 12,
    EventTypeModify = 13,
    GroupAccept = 14,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum AuditLogData {
    BindCreate {
        count: i32,
        kind: BindType,
    },
    BindModify {
        count: i32,
        kind: BindType,
    },
    BindDelete {
        count: i32,
        kind: BindType,
    },
    XPAdd {
        xp: i32,
        target_roblox_user: RobloxUserId,
    },
    XPRemove {
        xp: i32,
        target_roblox_user: RobloxUserId,
    },
    SetRank {
        target_roblox_user: RobloxUserId,
        group_id: GroupId,
        group_rank_id: u32,
    },
    XPSet {
        xp: i32,
        target_roblox_user: RobloxUserId,
    },
    DenylistCreate {
        kind: DenyListType,
    },
    DenylistDelete {
        count: i32,
    },
    EventLog {
        guild_event_id: i64,
    },
    SettingModify {
        setting: String,
        value: String,
    },
    EventTypeCreate {
        id: u32,
    },
    EventTypeDelete {
        id: u32,
    },
    EventTypeModify {
        id: u32,
    },
    GroupAccept {
        group_id: GroupId,
        target_roblox_user: RobloxUserId,
    },
}

impl TryFrom<tokio_postgres::Row> for AuditLog {
    type Error = tokio_postgres::Error;

    fn try_from(row: tokio_postgres::Row) -> Result<Self, tokio_postgres::Error> {
        let guild_id = row.try_get("guild_id")?;
        let kind = row.try_get("kind")?;
        let Json(metadata) = row.try_get("metadata")?;
        let user_id = row.try_get("user_id")?;
        let timestamp = row.try_get("timestamp")?;

        Ok(Self {
            kind,
            metadata,
            guild_id,
            user_id,
            timestamp,
        })
    }
}

impl<'a> FromSql<'a> for AuditLogKind {
    fn from_sql(
        ty: &Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let log_type = i32::from_sql(ty, raw)?;
        match log_type {
            1 => Ok(Self::BindCreate),
            2 => Ok(Self::BindModify),
            3 => Ok(Self::BindDelete),
            4 => Ok(Self::XPAdd),
            5 => Ok(Self::XPRemove),
            6 => Ok(Self::SetRank),
            7 => Ok(Self::XPSet),
            _ => unreachable!(),
        }
    }

    fn accepts(ty: &Type) -> bool {
        <i32 as FromSql>::accepts(ty)
    }
}

impl ToSql for AuditLogKind {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        i32::to_sql(&(*self as i32), ty, out)
    }

    fn accepts(ty: &Type) -> bool {
        <i32 as ToSql>::accepts(ty)
    }

    to_sql_checked!();
}
