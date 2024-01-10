use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use tokio_postgres::types::Json;

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
        let bypass_roles = row.try_get("bypass_roles")?;
        let unverified_roles = row.try_get("unverified_roles").unwrap_or_default();
        let verified_roles = row.try_get("verified_roles").unwrap_or_default();
        let rankbinds = row.try_get("rankbinds")?;
        let groupbinds = row.try_get("groupbinds")?;
        let assetbinds = row.try_get("assetbinds")?;
        let custombinds = row.try_get("custombinds")?;
        let deny_lists = row.try_get("deny_lists")?;
        let default_template = row.try_get("default_template").unwrap_or_default();

        Ok(Self {
            guild_id,
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
