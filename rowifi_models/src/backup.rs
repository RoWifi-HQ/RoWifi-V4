use serde::{Deserialize, Serialize};

use crate::{
    bind::{AssetType, Template, XPBind},
    deny_list::DenyList,
    events::EventType,
    guild::BypassRoleKind,
    roblox::id::{AssetId, GroupId},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct BackupGuild {
    pub bypass_roles: Vec<BackupBypassRole>,
    pub unverified_roles: Vec<String>,
    pub verified_roles: Vec<String>,
    pub rankbinds: Vec<BackupRankbind>,
    pub groupbinds: Vec<BackupGroupbind>,
    pub assetbinds: Vec<BackupAssetbind>,
    pub custombinds: Vec<BackupCustombind>,
    pub xp_binds: Vec<XPBind>,
    pub deny_lists: Vec<DenyList>,
    pub default_template: Template,
    pub update_on_join: bool,
    pub event_types: Vec<EventType>,
    pub auto_detection: bool,
    pub sync_xp_on_setrank: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BackupRankbind {
    /// The ID of the Roblox Group
    pub group_id: GroupId,
    /// The Discord Roles bound to this bind
    pub discord_roles: Vec<String>,
    /// The ID (0-255) of the rank
    pub group_rank_id: u32,
    /// The global rank ID
    pub roblox_rank_id: String,
    /// The priority of the bind. Used for determining the nickname
    pub priority: i32,
    /// The format of the nickname
    pub template: Template,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BackupGroupbind {
    /// The Id of the Roblox Group
    pub group_id: GroupId,
    /// The discord roles bound to the group
    pub discord_roles: Vec<String>,
    /// The number that decides whether this bind is chosen for the nickname
    pub priority: i32,
    /// The format of the nickname if this bind is chosen
    pub template: Template,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BackupCustombind {
    /// The ID of the Custom Bind
    pub custom_bind_id: u32,
    /// The discord roles bound to the custombind
    pub discord_roles: Vec<String>,
    /// The code of the bind
    pub code: String,
    /// The number that decides whether this bind is chosen for the nickname
    pub priority: i32,
    /// The format of the nickname if this bind is chosen
    pub template: Template,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BackupAssetbind {
    /// The ID of the asset
    pub asset_id: AssetId,
    /// The type of the Asset. Can be one of Asset, Badge, Gamepass
    pub asset_type: AssetType,
    /// The discord roles bounded to the asset
    pub discord_roles: Vec<String>,
    /// The number that decides whether this bind is chosen for the nickname
    pub priority: i32,
    /// The format of the nickname if this bind is chosen
    pub template: Template,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BackupBypassRole {
    pub role: String,
    pub kind: BypassRoleKind,
}
