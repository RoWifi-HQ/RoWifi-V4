use std::fmt::{Display, Formatter, Result as FmtResult};

pub enum Route {
    UserGroupRoles { user_id: u64 },
}

impl Display for Route {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Route::UserGroupRoles { user_id } => write!(
                f,
                "https://groups.roblox.com/v2/users/{user_id}/groups/roles"
            ),
        }
    }
}
