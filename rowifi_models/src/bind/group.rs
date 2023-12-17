use serde::{Deserialize, Serialize};

use crate::id::RoleId;

use super::Template;

#[derive(Debug, Deserialize, Serialize)]
pub struct Groupbind {
    /// The Id of the Roblox Group
    pub group_id: u64,
     /// The discord roles bound to the group
     pub discord_roles: Vec<RoleId>,
     /// The number that decides whether this bind is chosen for the nickname
     pub priority: i32,
     /// The format of the nickname if this bind is chosen
     pub template: Template,
}

impl Groupbind {
    pub fn discord_roles(&self) -> &[RoleId] {
        &self.discord_roles
    }
}