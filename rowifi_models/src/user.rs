use std::collections::HashMap;

use bitflags::bitflags;
use bytes::BytesMut;
use tokio_postgres::types::{to_sql_checked, FromSql, IsNull, ToSql, Type};

use crate::{
    id::{GuildId, UserId},
    roblox::id::UserId as RobloxUserId,
};

#[derive(Clone, Debug)]
pub struct RoUser {
    pub user_id: UserId,
    pub default_account_id: RobloxUserId,
    pub linked_accounts: HashMap<GuildId, RobloxUserId>,
    pub other_accounts: Vec<RobloxUserId>,
    pub flags: UserFlags,
}

#[derive(Clone, Debug)]
pub struct PatreonUser {
    pub user_id: UserId,
    pub patreon_id: i64,
    pub premium_servers: Vec<GuildId>,
    pub transferred_from: Option<UserId>,
    pub transferred_to: Option<UserId>,
}

bitflags! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct UserFlags: i64 {
        const NONE = 0;
        const PATREON_ALPHA = 1;
        const PATREON_BETA = 1 << 1;
        const STAFF = 1 << 2;
        const PARTNER = 1 << 3;
    }
}

impl TryFrom<tokio_postgres::Row> for RoUser {
    type Error = tokio_postgres::Error;

    fn try_from(row: tokio_postgres::Row) -> Result<Self, Self::Error> {
        let user_id = row.try_get("user_id")?;
        let default_account_id = row.try_get("default_account_id")?;
        let linked_accounts_sql: HashMap<String, Option<String>> =
            row.try_get("linked_accounts")?;
        let other_accounts_sql: Vec<i64> = row.try_get("other_accounts")?;
        let flags = row.try_get("flags")?;

        let linked_accounts = linked_accounts_sql
            .into_iter()
            .map(|(k, v)| {
                let discord_id = k.parse::<u64>().map(GuildId::new).unwrap();
                let roblox_id = v.unwrap().parse::<u64>().map(RobloxUserId).unwrap();
                (discord_id, roblox_id)
            })
            .collect::<HashMap<_, _>>();

        #[allow(clippy::cast_sign_loss)]
        let other_accounts = other_accounts_sql
            .into_iter()
            .map(|a| RobloxUserId(a as u64))
            .collect::<Vec<_>>();

        Ok(Self {
            user_id,
            default_account_id,
            linked_accounts,
            other_accounts,
            flags,
        })
    }
}

impl TryFrom<tokio_postgres::Row> for PatreonUser {
    type Error = tokio_postgres::Error;

    fn try_from(row: tokio_postgres::Row) -> Result<Self, Self::Error> {
        let user_id = row.try_get("user_id")?;
        let patreon_id = row.try_get("patreon_id")?;
        let premium_servers = row.try_get("premium_servers")?;
        let transferred_from = row.try_get("transferred_from")?;
        let transferred_to = row.try_get("transferred_to")?;

        Ok(Self {
            user_id,
            patreon_id,
            premium_servers,
            transferred_from,
            transferred_to,
        })
    }
}

impl<'a> FromSql<'a> for UserFlags {
    fn from_sql(
        ty: &Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let bits = i64::from_sql(ty, raw)?;
        Ok(Self::from_bits_truncate(bits))
    }

    fn accepts(ty: &Type) -> bool {
        <i64 as FromSql>::accepts(ty)
    }
}

impl ToSql for UserFlags {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        i64::to_sql(&self.bits(), ty, out)
    }

    fn accepts(ty: &Type) -> bool {
        <i64 as ToSql>::accepts(ty)
    }

    to_sql_checked!();
}
