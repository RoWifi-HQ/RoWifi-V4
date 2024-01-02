mod asset;
mod group;
mod rank;
mod template;

pub use asset::{AssetType, Assetbind};
pub use group::Groupbind;
pub use rank::Rankbind;
pub use template::Template;

use crate::id::RoleId;

pub enum Bind {
    Rank(Rankbind),
    Group(Groupbind),
    Asset(Assetbind),
}

impl Bind {
    pub fn discord_roles(&self) -> &[RoleId] {
        match self {
            Self::Rank(r) => r.discord_roles(),
            Self::Group(g) => g.discord_roles(),
            Self::Asset(a) => a.discord_roles(),
        }
    }

    pub fn priority(&self) -> i32 {
        match self {
            Self::Rank(r) => r.priority,
            Self::Group(g) => g.priority,
            Self::Asset(a) => a.priority,
        }
    }
}
