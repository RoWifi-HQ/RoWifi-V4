use std::fmt::{Display, Formatter, Result as FmtResult};

pub enum Route {
    GetUserGroupRoles { user_id: u64 },
    GetUser { user_id: u64 }
}

impl Display for Route {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Route::GetUserGroupRoles { user_id } => write!(
                f,
                "https://groups.roblox.com/v2/users/{user_id}/groups/roles"
            ),
            Route::GetUser { user_id } => write!(f, "https://apis.roblox.com/cloud/v2/users/{user_id}")
        }
    }
}
