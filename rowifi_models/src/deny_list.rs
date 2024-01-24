use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt::{Display, Formatter, Result as FmtResult};

use crate::roblox::id::{GroupId, UserId};

#[derive(Clone, Debug)]
pub struct DenyList {
    pub id: u32,
    pub reason: String,
    pub action_type: DenyListActionType,
    pub data: DenyListData,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DenyListData {
    User(UserId),
    Group(GroupId),
    // TODO: Add custom denylist
}

#[derive(Clone, Copy, Debug, Deserialize_repr, Eq, PartialEq, Serialize_repr)]
#[repr(u8)]
pub enum DenyListType {
    User = 0,
    Group = 1,
    Custom = 2,
}

#[derive(
    Clone, Copy, Debug, Default, Deserialize_repr, Eq, Ord, PartialEq, PartialOrd, Serialize_repr,
)]
#[repr(u8)]
pub enum DenyListActionType {
    #[default]
    None = 0,
    Kick = 1,
    Ban = 2,
}

#[derive(Deserialize, Serialize)]
struct DenyListIntermediary {
    pub id: u32,
    pub reason: String,
    pub kind: DenyListType,
    pub action_type: DenyListActionType,
    pub user_id: Option<UserId>,
    pub group_id: Option<GroupId>,
}

impl DenyList {
    #[must_use]
    pub const fn kind(&self) -> DenyListType {
        match self.data {
            DenyListData::User(_) => DenyListType::User,
            DenyListData::Group(_) => DenyListType::Group,
        }
    }
}

impl<'de> Deserialize<'de> for DenyList {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let intermediary = DenyListIntermediary::deserialize(deserializer)?;
        let data = match intermediary.kind {
            DenyListType::User => DenyListData::User(intermediary.user_id.unwrap()),
            DenyListType::Group => DenyListData::Group(intermediary.group_id.unwrap()),
            DenyListType::Custom => todo!(),
        };
        Ok(DenyList {
            id: intermediary.id,
            reason: intermediary.reason,
            action_type: intermediary.action_type,
            data,
        })
    }
}

impl Serialize for DenyList {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (user_id, group_id) = match &self.data {
            DenyListData::User(u) => (Some(*u), None),
            DenyListData::Group(g) => (None, Some(*g)),
        };
        let intermediary = DenyListIntermediary {
            id: self.id,
            reason: self.reason.clone(),
            kind: self.kind(),
            action_type: self.action_type,
            user_id,
            group_id,
        };
        intermediary.serialize(serializer)
    }
}

impl Display for DenyListType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::User => f.write_str("User"),
            Self::Group => f.write_str("Group"),
            Self::Custom => f.write_str("Custom"),
        }
    }
}
