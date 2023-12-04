use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};
use twilight_model::id::{
    marker::{ChannelMarker, GuildMarker, RoleMarker, UserMarker},
    Id,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct GuildId(pub Id<GuildMarker>);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct UserId(pub Id<UserMarker>);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct RoleId(pub Id<RoleMarker>);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ChannelId(pub Id<ChannelMarker>);

impl GuildId {
    #[must_use]
    pub fn new(n: u64) -> Self {
        Self(Id::new(n))
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl UserId {
    #[must_use]
    pub fn new(n: u64) -> Self {
        Self(Id::new(n))
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl RoleId {
    #[must_use]
    pub fn new(n: u64) -> Self {
        Self(Id::new(n))
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl ChannelId {
    #[must_use]
    pub fn new(n: u64) -> Self {
        Self(Id::new(n))
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl Display for GuildId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(&self.0, f)
    }
}

impl Display for UserId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(&self.0, f)
    }
}

impl Display for RoleId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(&self.0, f)
    }
}

impl Display for ChannelId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(&self.0, f)
    }
}
