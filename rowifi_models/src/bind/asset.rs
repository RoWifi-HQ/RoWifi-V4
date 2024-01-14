use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt::{Display, Formatter, Result as FmtResult};

use crate::{id::RoleId, roblox::id::AssetId};

use super::Template;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Assetbind {
    /// The ID of the asset
    pub asset_id: AssetId,
    /// The type of the Asset. Can be one of Asset, Badge, Gamepass
    pub asset_type: AssetType,
    /// The discord roles bounded to the asset
    pub discord_roles: Vec<RoleId>,
    /// The number that decides whether this bind is chosen for the nickname
    pub priority: i32,
    /// The format of the nickname if this bind is chosen
    pub template: Template,
}

#[derive(Clone, Copy, Debug, Default, Deserialize_repr, Eq, PartialEq, Serialize_repr)]
#[repr(u8)]
pub enum AssetType {
    #[default]
    Asset = 0,
    Badge = 1,
    Gamepass = 2,
}

impl Assetbind {
    #[must_use]
    pub fn discord_roles(&self) -> &[RoleId] {
        &self.discord_roles
    }
}

impl Display for AssetType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match *self {
            AssetType::Asset => f.write_str("Asset"),
            AssetType::Badge => f.write_str("Badge"),
            AssetType::Gamepass => f.write_str("Gamepass"),
        }
    }
}
