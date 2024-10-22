use bytes::BytesMut;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt::{Display, Formatter, Result as FmtResult};
use tokio_postgres::types::{to_sql_checked, FromSql, IsNull, Json, ToSql, Type};

use crate::{
    bind::{Assetbind, Custombind, Groupbind, Rankbind, Template, XPBind},
    deny_list::DenyList,
    events::EventType,
    id::{ChannelId, GuildId, RoleId},
    roblox::id::GroupId,
};

#[derive(Debug)]
pub struct RoGuild {
    pub guild_id: GuildId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartialRoGuild {
    pub guild_id: GuildId,
    pub kind: Option<GuildType>,
    pub bypass_roles: Vec<BypassRole>,
    pub unverified_roles: Vec<RoleId>,
    pub verified_roles: Vec<RoleId>,
    pub rankbinds: Vec<Rankbind>,
    pub groupbinds: Vec<Groupbind>,
    pub assetbinds: Vec<Assetbind>,
    pub custombinds: Vec<Custombind>,
    pub deny_lists: Vec<DenyList>,
    pub default_template: Option<Template>,
    pub update_on_join: Option<bool>,
    pub event_types: Vec<EventType>,
    pub auto_detection: Option<bool>,
    pub xp_binds: Vec<XPBind>,
    pub sync_xp_on_setrank: Option<bool>,
    pub registered_groups: Vec<GroupId>,
    pub sticky_roles: Vec<RoleId>,
    pub log_channel: Option<ChannelId>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize_repr, Eq, PartialEq, Serialize_repr)]
#[repr(u32)]
pub enum GuildType {
    #[default]
    Free = 0,
    Alpha = 1,
    Beta = 2,
    Gamma = 3,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Serialize, PartialEq)]
pub struct BypassRole {
    pub role_id: RoleId,
    pub kind: BypassRoleKind,
}

#[derive(Clone, Copy, Debug, Deserialize_repr, Eq, PartialEq, Serialize_repr)]
#[repr(u8)]
pub enum BypassRoleKind {
    Roles = 0,
    Nickname = 1,
    All = 2,
}

impl PartialRoGuild {
    #[must_use]
    pub fn new(guild_id: GuildId) -> Self {
        Self {
            guild_id,
            kind: Some(GuildType::Free),
            bypass_roles: Vec::new(),
            unverified_roles: Vec::new(),
            verified_roles: Vec::new(),
            rankbinds: Vec::new(),
            groupbinds: Vec::new(),
            assetbinds: Vec::new(),
            custombinds: Vec::new(),
            deny_lists: Vec::new(),
            default_template: None,
            update_on_join: None,
            event_types: Vec::new(),
            auto_detection: None,
            xp_binds: Vec::new(),
            sync_xp_on_setrank: None,
            registered_groups: Vec::new(),
            sticky_roles: Vec::new(),
            log_channel: None,
        }
    }
}

impl TryFrom<tokio_postgres::Row> for PartialRoGuild {
    type Error = tokio_postgres::Error;

    fn try_from(row: tokio_postgres::Row) -> Result<Self, Self::Error> {
        let guild_id = row.try_get("guild_id")?;
        let kind = row.try_get("kind").ok();
        let bypass_roles = row
            .try_get("bypass_roles")
            .unwrap_or_else(|_| Json(Vec::new()));
        let unverified_roles = row.try_get("unverified_roles").unwrap_or_default();
        let verified_roles = row.try_get("verified_roles").unwrap_or_default();
        let rankbinds = row
            .try_get("rankbinds")
            .unwrap_or_else(|_| Json(Vec::new()));
        let groupbinds = row
            .try_get("groupbinds")
            .unwrap_or_else(|_| Json(Vec::new()));
        let assetbinds = row
            .try_get("assetbinds")
            .unwrap_or_else(|_| Json(Vec::new()));
        let custombinds = row
            .try_get("custombinds")
            .unwrap_or_else(|_| Json(Vec::new()));
        let deny_lists = row
            .try_get("deny_lists")
            .unwrap_or_else(|_| Json(Vec::new()));
        let default_template = row.try_get("default_template").unwrap_or_default();
        let update_on_join = row.try_get("update_on_join").unwrap_or_default();
        let event_types = row
            .try_get("event_types")
            .unwrap_or_else(|_| Json(Vec::new()));
        let auto_detection = row.try_get("auto_detection").unwrap_or_default();
        let xp_binds = row.try_get("xp_binds").unwrap_or_else(|_| Json(Vec::new()));
        let sync_xp_on_setrank = row.try_get("sync_xp_on_setrank").unwrap_or_default();
        let registered_groups = row
            .try_get("registered_groups")
            .unwrap_or_else(|_| Json(Vec::new()));
        let sticky_roles = row.try_get("sticky_roles").unwrap_or_default();
        let log_channel = row.try_get("channel_id").ok();

        Ok(Self {
            guild_id,
            kind,
            bypass_roles: bypass_roles.0,
            unverified_roles,
            verified_roles,
            rankbinds: rankbinds.0,
            groupbinds: groupbinds.0,
            assetbinds: assetbinds.0,
            custombinds: custombinds.0,
            deny_lists: deny_lists.0,
            default_template,
            update_on_join,
            event_types: event_types.0,
            auto_detection,
            xp_binds: xp_binds.0,
            sync_xp_on_setrank,
            registered_groups: registered_groups.0,
            sticky_roles,
            log_channel,
        })
    }
}

impl ToSql for GuildType {
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

impl<'a> FromSql<'a> for GuildType {
    fn from_sql(
        ty: &Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        match i32::from_sql(ty, raw)? {
            0 => Ok(GuildType::Free),
            1 => Ok(GuildType::Alpha),
            2 => Ok(GuildType::Beta),
            3 => Ok(GuildType::Gamma),
            _ => unreachable!(),
        }
    }

    fn accepts(ty: &Type) -> bool {
        <i32 as FromSql>::accepts(ty)
    }
}

impl Display for GuildType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Free => f.write_str("Free"),
            Self::Alpha => f.write_str("Alpha"),
            Self::Beta => f.write_str("Beta"),
            Self::Gamma => f.write_str("Gamma"),
        }
    }
}
