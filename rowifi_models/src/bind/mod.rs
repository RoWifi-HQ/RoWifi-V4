mod rank;
mod template;
mod group;
mod asset;

pub use template::Template;
pub use rank::Rankbind;
pub use group::Groupbind;
pub use asset::{AssetType, Assetbind};

use crate::id::RoleId;

pub enum Bind {
    Rank(Rankbind),
    Group(Groupbind),
    Asset(Assetbind)
}

impl Bind {
    pub fn discord_roles(&self) -> &[RoleId] {
        match self {
            Self::Rank(r) => r.discord_roles(),
            Self::Group(g) => g.discord_roles(),
            Self::Asset(a) => a.discord_roles(),
        }
    }
}