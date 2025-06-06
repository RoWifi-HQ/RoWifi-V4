use std::fmt::{Display, Formatter, Result as FmtResult};

pub enum Route<'a> {
    CreateDatastoreEntry {
        universe_id: u64,
        datastore_id: &'a str,
        entry_id: &'a str,
    },
    DeleteDatastoreEntry {
        universe_id: u64,
        datastore_id: &'a str,
        entry_id: &'a str,
    },
    GetDatastoreEntry {
        universe_id: u64,
        datastore_id: &'a str,
        entry_id: &'a str,
        revision_id: Option<&'a str>,
    },
    GetGroup {
        group_id: u64,
    },
    GetUniverse {
        universe_id: u64,
    },
    GetUserGroupRoles {
        user_id: u64,
    },
    GetUser {
        user_id: u64,
    },
    GetUsers,
    GetUserByUsernames,
    GetUserThumbail {
        user_id: u64,
    },
    ListDatastores {
        universe_id: u64,
        query: &'a str,
    },
    ListDatastoreEntries {
        universe_id: u64,
        datastore_id: &'a str,
        page_token: &'a str,
        page_size: u32,
        filter: Option<&'a str>,
    },
    ListDatastoreEntryRevisions {
        universe_id: u64,
        datastore_id: &'a str,
        entry_id: &'a str,
        page_token: &'a str,
        page_size: u32,
    },
    ListInventoryItems {
        user_id: u64,
        filter: &'a str,
    },
    ListGroupRanks {
        group_id: u64,
    },
    OAuthUserInfo,
    PublishUniverseMessage {
        universe_id: u64,
    },
    UpdateDatastoreEntry {
        universe_id: u64,
        datastore_id: &'a str,
        entry_id: &'a str,
    },
}

impl Display for Route<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Route::CreateDatastoreEntry { universe_id, datastore_id, entry_id } => write!(f, "https://apis.roblox.com/cloud/v2/universes/{universe_id}/data-stores/{datastore_id}/entries?id={entry_id}"),
            Route::DeleteDatastoreEntry { universe_id, datastore_id, entry_id } => write!(f, "https://apis.roblox.com/cloud/v2/universes/{universe_id}/data-stores/{datastore_id}/entries/{entry_id}"), 
            Route::GetDatastoreEntry { universe_id, datastore_id, entry_id, revision_id } => write!(
                f,
                "https://apis.roblox.com/cloud/v2/universes/{universe_id}/data-stores/{datastore_id}/entries/{entry_id}@{}",
                revision_id.unwrap_or("latest")
            ),
            Route::GetGroup { group_id } => write!(f, "https://apis.roblox.com/cloud/v2/groups/{group_id}"),
            Route::GetUniverse { universe_id } => write!(f, "https://apis.roblox.com/cloud/v2/universes/{universe_id}"),
            Route::GetUserGroupRoles { user_id } => write!(
                f,
                "https://groups.roblox.com/v2/users/{user_id}/groups/roles"
            ),
            Route::GetUser { user_id } => write!(f, "https://apis.roblox.com/cloud/v2/users/{user_id}"),
            Route::GetUsers => write!(f, "https://users.roblox.com/v1/users"),
            Route::GetUserByUsernames => write!(f, "https://users.roblox.com/v1/usernames/users"),
            Route::GetUserThumbail { user_id } => write!(f, "https://apis.roblox.com/cloud/v2/users/{user_id}:generateThumbnail?size=420&format=PNG"),
            Route::ListDatastores { universe_id, query } => write!(f, "https://apis.roblox.com/cloud/v2/universes/{universe_id}/data-stores?maxPageSize=100&{query}"),
            Route::ListDatastoreEntries { universe_id, datastore_id, page_token, page_size,filter } => {
                // Encoding the entire filter causes it to give an invalid filter. So, just pre-encode the quotes.
                let filter = filter.map(|f| format!("&filter=id.startsWith(%22{f}%22)")).unwrap_or_default();
                write!(f, "https://apis.roblox.com/cloud/v2/universes/{universe_id}/data-stores/{datastore_id}/entries?maxPageSize={page_size}&pageToken={page_token}{filter}")
            },
            Route::ListDatastoreEntryRevisions { universe_id, datastore_id, entry_id, page_token, page_size } => {
                write!(f, "https://apis.roblox.com/cloud/v2/universes/{universe_id}/data-stores/{datastore_id}/entries/{entry_id}:listRevisions?maxPageSize={page_size}&pageToken={page_token}")
            },
            Route::ListInventoryItems { user_id, filter } => write!(f, "https://apis.roblox.com/cloud/v2/users/{user_id}/inventory-items?maxPageSize=100&{filter}"),
            Route::ListGroupRanks { group_id } => write!(f, "https://apis.roblox.com/cloud/v2/groups/{group_id}/roles?maxPageSize=20"),
            Route::OAuthUserInfo => write!(f, "https://apis.roblox.com/oauth/v1/userinfo"),
            Route::PublishUniverseMessage { universe_id } => write!(f, "https://apis.roblox.com/cloud/v2/universes/{universe_id}:publishMessage"),
            Route::UpdateDatastoreEntry { universe_id, datastore_id, entry_id } => write!(f, "https://apis.roblox.com/cloud/v2/universes/{universe_id}/data-stores/{datastore_id}/entries/{entry_id}")
        }
    }
}
