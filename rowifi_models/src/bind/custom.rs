use serde::{Deserialize, Serialize};

use super::Template;
use crate::id::RoleId;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Custombind {
    /// The ID of the Custom Bind
    pub custom_bind_id: u32,
    /// The discord roles bound to the custombind
    pub discord_roles: Vec<RoleId>,
    /// The code of the bind
    pub code: String,
    /// The number that decides whether this bind is chosen for the nickname
    pub priority: i32,
    /// The format of the nickname if this bind is chosen
    pub template: Template,
}

impl Custombind {
    #[must_use]
    pub fn discord_roles(&self) -> &[RoleId] {
        &self.discord_roles
    }

    #[must_use]
    pub fn evaluate(&self) -> bool {
        // TODO: Custombind code evaluation
        false
    }
}
