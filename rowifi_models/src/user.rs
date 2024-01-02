use std::collections::HashMap;

use crate::{
    id::{GuildId, UserId},
    roblox::id::UserId as RobloxUserId,
};

#[derive(Debug)]
pub struct RoUser {
    pub user_id: UserId,
    pub default_account_id: RobloxUserId,
    pub linked_accounts: HashMap<GuildId, RobloxUserId>,
}

impl TryFrom<tokio_postgres::Row> for RoUser {
    type Error = tokio_postgres::Error;

    fn try_from(row: tokio_postgres::Row) -> Result<Self, Self::Error> {
        let user_id = row.try_get("user_id")?;
        let default_account_id = row.try_get("default_account_id")?;
        let linked_accounts_sql: HashMap<String, Option<String>> =
            row.try_get("linked_accounts")?;

        let linked_accounts = linked_accounts_sql
            .into_iter()
            .map(|(k, v)| {
                let discord_id = k.parse::<u64>().map(GuildId::new).unwrap();
                let roblox_id = v.unwrap().parse::<u64>().map(RobloxUserId).unwrap();
                (discord_id, roblox_id)
            })
            .collect::<HashMap<_, _>>();

        Ok(Self {
            user_id,
            default_account_id,
            linked_accounts,
        })
    }
}
