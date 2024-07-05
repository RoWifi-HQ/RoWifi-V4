mod asset;
mod custom;
mod group;
mod rank;
mod template;

use serde_repr::{Deserialize_repr, Serialize_repr};

pub use asset::{AssetType, Assetbind};
pub use custom::Custombind;
pub use group::Groupbind;
pub use rank::Rankbind;
pub use template::Template;

use crate::id::RoleId;

pub enum Bind {
    Rank(Rankbind),
    Group(Groupbind),
    Asset(Assetbind),
    Custom(Custombind),
}

#[derive(Clone, Copy, Debug, Deserialize_repr, Eq, PartialEq, Serialize_repr)]
#[repr(u8)]
pub enum BindType {
    Rank = 0,
    Group = 1,
    Custom = 2,
    Asset = 3,
    XP = 4,
}

impl Bind {
    #[must_use]
    pub fn discord_roles(&self) -> &[RoleId] {
        match self {
            Self::Rank(r) => r.discord_roles(),
            Self::Group(g) => g.discord_roles(),
            Self::Asset(a) => a.discord_roles(),
            Self::Custom(c) => c.discord_roles(),
        }
    }

    #[must_use]
    pub fn priority(&self) -> i32 {
        match self {
            Self::Rank(r) => r.priority,
            Self::Group(g) => g.priority,
            Self::Asset(a) => a.priority,
            Self::Custom(c) => c.priority,
        }
    }
}
