use std::fmt::{Display, Formatter, Result as FmtResult};

pub enum Route {
    GetGroup { group_id: u64 },
    GetUserGroupRoles { user_id: u64 },
    GetUser { user_id: u64 },
    ListInventoryItems { user_id: u64, filter: String },
    ListGroupRanks { group_id: u64 },
}

impl Display for Route {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Route::GetGroup { group_id } => write!(f, "https://apis.roblox.com/cloud/v2/groups/{group_id}"),
            Route::GetUserGroupRoles { user_id } => write!(
                f,
                "https://groups.roblox.com/v2/users/{user_id}/groups/roles"
            ),
            Route::GetUser { user_id } => write!(f, "https://apis.roblox.com/cloud/v2/users/{user_id}"),
            Route::ListInventoryItems { user_id, filter } => write!(f, "https://apis.roblox.com/cloud/v2/users/{user_id}/inventory-items?maxPageSize=100&{filter}"),
            Route::ListGroupRanks { group_id } => write!(f, "https://apis.roblox.com/cloud/v2/groups/{group_id}/roles?maxPageSize=20")
        }
    }
}
