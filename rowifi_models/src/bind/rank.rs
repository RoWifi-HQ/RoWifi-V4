use serde::{Deserialize, Serialize};

use crate::{id::RoleId, roblox::id::GroupId};

use super::Template;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Rankbind {
    /// The ID of the Roblox Group
    pub group_id: GroupId,
    /// The Discord Roles bound to this bind
    pub discord_roles: Vec<RoleId>,
    /// The ID (0-255) of the rank
    pub group_rank_id: u32,
    /// The global rank ID
    pub roblox_rank_id: String,
    /// The priority of the bind. Used for determining the nickname
    pub priority: i32,
    /// The format of the nickname
    pub template: Template,
}

impl Rankbind {
    #[must_use]
    pub fn discord_roles(&self) -> &[RoleId] {
        &self.discord_roles
    }
}
