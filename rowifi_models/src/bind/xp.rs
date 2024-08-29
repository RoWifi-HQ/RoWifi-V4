use serde::{Deserialize, Serialize};

use crate::roblox::id::GroupId;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct XPBind {
    pub group_id: GroupId,
    pub group_rank_id: u32,
    pub roblox_rank_id: String,
    /// XP required to reach this rank
    pub xp: i64,
}