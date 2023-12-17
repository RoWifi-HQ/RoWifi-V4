use tokio_postgres::types::Json;

use crate::{id::{GuildId, RoleId}, bind::{Rankbind, Groupbind, Assetbind}};

pub struct RoGuild {
    pub guild_id: GuildId
}

#[derive(Debug)]
pub struct PartialRoGuild {
    pub guild_id: GuildId,
    pub bypass_roles: Vec<RoleId>,
    pub unverified_roles: Vec<RoleId>,
    pub verified_roles: Vec<RoleId>,
    pub rankbinds: Json<Vec<Rankbind>>,
    pub groupbinds: Json<Vec<Groupbind>>,
    pub assetbinds: Json<Vec<Assetbind>>,
}

impl PartialRoGuild {
    pub fn new(guild_id: GuildId) -> Self {
        Self {
            guild_id,
            bypass_roles: Vec::new(),
            unverified_roles: Vec::new(),
            verified_roles: Vec::new(),
            rankbinds: Json(Vec::new()),
            groupbinds: Json(Vec::new()),
            assetbinds: Json(Vec::new())
        }
    }
}

impl TryFrom<tokio_postgres::Row> for PartialRoGuild {
    type Error = tokio_postgres::Error;

    fn try_from(row: tokio_postgres::Row) -> Result<Self, Self::Error> {
        let guild_id = row.try_get("guild_id")?;
        let bypass_roles = row.try_get("bypass_roles").unwrap_or_default();
        let unverified_roles = row.try_get("unverified_roles").unwrap_or_default();
        let verified_roles = row.try_get("verified_roles").unwrap_or_default();
        let rankbinds = row.try_get("rankbinds")?;
        let groupbinds = row.try_get("groupbinds")?;
        let assetbinds = row.try_get("assetbinds")?;

        Ok(Self {
            guild_id,
            bypass_roles,
            unverified_roles,
            verified_roles,
            rankbinds,
            groupbinds,
            assetbinds,
        })
    }
}