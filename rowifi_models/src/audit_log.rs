use bytes::BytesMut;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use serde_repr::{Deserialize_repr, Serialize_repr};
use tokio_postgres::types::{to_sql_checked, FromSql, IsNull, Json, ToSql, Type};

use crate::{
    bind::BindType,
    deny_list::DenyListType,
    id::{GuildId, UserId},
    roblox::id::{GroupId, UserId as RobloxUserId},
};

#[derive(Clone, Debug, Serialize)]
pub struct AuditLog {
    pub kind: AuditLogKind,
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
    GroupDecline = 15,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum AuditLogData {
    BindCreate(BindCreate),
    BindModify(BindModify),
    BindDelete(BindDelete),
    XPAdd(XPAdd),
    XPRemove(XPRemove),
    SetRank(SetRank),
    XPSet(XPSet),
    DenylistCreate(DenylistCreate),
    DenylistDelete(DenylistDelete),
    EventLog(EventLog),
    SettingModify(SettingModify),
    EventTypeCreate(EventTypeCreate),
    EventTypeDelete(EventTypeDelete),
    EventTypeModify(EventTypeModify),
    GroupAccept(GroupAccept),
    GroupDecline(GroupDecline),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BindCreate {
    pub count: i32,
    pub kind: BindType,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BindModify {
    pub count: i32,
    pub kind: BindType,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BindDelete {
    pub count: i32,
    pub kind: BindType,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct XPAdd {
    pub xp: i32,
    pub target_roblox_user: RobloxUserId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct XPRemove {
    pub xp: i32,
    pub target_roblox_user: RobloxUserId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetRank {
    pub target_roblox_user: RobloxUserId,
    pub group_id: GroupId,
    pub group_rank_id: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct XPSet {
    pub xp: i32,
    pub target_roblox_user: RobloxUserId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DenylistCreate {
    pub kind: DenyListType,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DenylistDelete {
    pub count: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventLog {
    pub guild_event_id: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SettingModify {
    pub setting: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventTypeCreate {
    pub id: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventTypeModify {
    pub id: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventTypeDelete {
    pub id: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GroupAccept {
    pub group_id: GroupId,
    pub target_roblox_user: RobloxUserId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GroupDecline {
    pub group_id: GroupId,
    pub target_roblox_user: RobloxUserId,
}

pub enum AuditLogDeserializeError {
    Serde(serde_json::Error),
    Postgres(tokio_postgres::Error),
}

impl TryFrom<tokio_postgres::Row> for AuditLog {
    type Error = AuditLogDeserializeError;

    fn try_from(row: tokio_postgres::Row) -> Result<Self, Self::Error> {
        let guild_id: Option<GuildId> = row.try_get("guild_id")?;
        let kind: AuditLogKind = row.try_get("kind")?;
        let metadata: Json<Box<RawValue>> = row.try_get("metadata")?;
        let user_id: Option<UserId> = row.try_get("user_id")?;
        let timestamp: DateTime<Utc> = row.try_get("timestamp")?;

        let metadata: AuditLogData = match kind {
            AuditLogKind::BindCreate => {
                AuditLogData::BindCreate(BindCreate::deserialize(metadata.0.as_ref())?)
            }
            AuditLogKind::BindModify => {
                AuditLogData::BindModify(BindModify::deserialize(metadata.0.as_ref())?)
            }
            AuditLogKind::BindDelete => {
                AuditLogData::BindDelete(BindDelete::deserialize(metadata.0.as_ref())?)
            }
            AuditLogKind::XPAdd => AuditLogData::XPAdd(XPAdd::deserialize(metadata.0.as_ref())?),
            AuditLogKind::XPRemove => {
                AuditLogData::XPRemove(XPRemove::deserialize(metadata.0.as_ref())?)
            }
            AuditLogKind::SetRank => {
                AuditLogData::SetRank(SetRank::deserialize(metadata.0.as_ref())?)
            }
            AuditLogKind::XPSet => AuditLogData::XPSet(XPSet::deserialize(metadata.0.as_ref())?),
            AuditLogKind::DenylistCreate => {
                AuditLogData::DenylistCreate(DenylistCreate::deserialize(metadata.0.as_ref())?)
            }
            AuditLogKind::DenylistDelete => {
                AuditLogData::DenylistDelete(DenylistDelete::deserialize(metadata.0.as_ref())?)
            }
            AuditLogKind::EventLog => {
                AuditLogData::EventLog(EventLog::deserialize(metadata.0.as_ref())?)
            }
            AuditLogKind::SettingModify => {
                AuditLogData::SettingModify(SettingModify::deserialize(metadata.0.as_ref())?)
            }
            AuditLogKind::EventTypeCreate => {
                AuditLogData::EventTypeCreate(EventTypeCreate::deserialize(metadata.0.as_ref())?)
            }
            AuditLogKind::EventTypeModify => {
                AuditLogData::EventTypeModify(EventTypeModify::deserialize(metadata.0.as_ref())?)
            }
            AuditLogKind::GroupAccept => {
                AuditLogData::GroupAccept(GroupAccept::deserialize(metadata.0.as_ref())?)
            }
            AuditLogKind::GroupDecline => {
                AuditLogData::GroupDecline(GroupDecline::deserialize(metadata.0.as_ref())?)
            }
        };

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
            8 => Ok(Self::DenylistCreate),
            9 => Ok(Self::DenylistDelete),
            10 => Ok(Self::EventLog),
            11 => Ok(Self::SettingModify),
            12 => Ok(Self::EventTypeCreate),
            13 => Ok(Self::EventTypeModify),
            14 => Ok(Self::GroupAccept),
            15 => Ok(Self::GroupDecline),
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

impl From<serde_json::Error> for AuditLogDeserializeError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serde(err)
    }
}

impl From<tokio_postgres::Error> for AuditLogDeserializeError {
    fn from(err: tokio_postgres::Error) -> Self {
        Self::Postgres(err)
    }
}
