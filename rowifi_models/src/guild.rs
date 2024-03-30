use bytes::BytesMut;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use tokio_postgres::types::{to_sql_checked, FromSql, IsNull, Json, ToSql, Type};

use crate::{
    bind::{Assetbind, Custombind, Groupbind, Rankbind, Template},
    deny_list::DenyList,
    id::{GuildId, RoleId},
};

#[derive(Debug)]
pub struct RoGuild {
    pub guild_id: GuildId,
}

#[derive(Debug)]
pub struct PartialRoGuild {
    pub guild_id: GuildId,
    pub kind: GuildType,
    pub bypass_roles: Json<Vec<BypassRole>>,
    pub unverified_roles: Vec<RoleId>,
    pub verified_roles: Vec<RoleId>,
    pub rankbinds: Json<Vec<Rankbind>>,
    pub groupbinds: Json<Vec<Groupbind>>,
    pub assetbinds: Json<Vec<Assetbind>>,
    pub custombinds: Json<Vec<Custombind>>,
    pub deny_lists: Json<Vec<DenyList>>,
    pub default_template: Option<Template>,
}

#[derive(Clone, Copy, Debug, Deserialize_repr, Eq, PartialEq, Serialize_repr)]
#[repr(u32)]
pub enum GuildType {
    Free = 0,
    Alpha = 1,
    Beta = 2,
    Gamma = 3,
}

#[derive(Debug, Deserialize, Serialize)]
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
            kind: GuildType::Free,
            bypass_roles: Json(Vec::new()),
            unverified_roles: Vec::new(),
            verified_roles: Vec::new(),
            rankbinds: Json(Vec::new()),
            groupbinds: Json(Vec::new()),
            assetbinds: Json(Vec::new()),
            custombinds: Json(Vec::new()),
            deny_lists: Json(Vec::new()),
            default_template: None,
        }
    }
}

impl TryFrom<tokio_postgres::Row> for PartialRoGuild {
    type Error = tokio_postgres::Error;

    fn try_from(row: tokio_postgres::Row) -> Result<Self, Self::Error> {
        let guild_id = row.try_get("guild_id")?;
        let kind = row.try_get("kind")?;
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

        Ok(Self {
            guild_id,
            kind,
            bypass_roles,
            unverified_roles,
            verified_roles,
            rankbinds,
            groupbinds,
            assetbinds,
            custombinds,
            deny_lists,
            default_template,
        })
    }
}

impl ToSql for GuildType {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        u32::to_sql(&(*self as u32), ty, out)
    }

    fn accepts(ty: &Type) -> bool {
        <u32 as ToSql>::accepts(ty)
    }

    to_sql_checked!();
}

impl<'a> FromSql<'a> for GuildType {
    fn from_sql(
        ty: &Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        match u32::from_sql(ty, raw)? {
            0 => Ok(GuildType::Free),
            1 => Ok(GuildType::Alpha),
            2 => Ok(GuildType::Beta),
            3 => Ok(GuildType::Gamma),
            _ => unreachable!(),
        }
    }

    fn accepts(ty: &Type) -> bool {
        <String as FromSql>::accepts(ty)
    }
}
