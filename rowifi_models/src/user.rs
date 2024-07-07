use bitflags::bitflags;
use bytes::BytesMut;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::HashMap;
use tokio_postgres::types::{to_sql_checked, FromSql, IsNull, ToSql, Type};

use crate::{
    id::{GuildId, UserId},
    roblox::id::UserId as RobloxUserId,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RoUser {
    pub user_id: UserId,
    pub default_account_id: RobloxUserId,
    pub linked_accounts: HashMap<GuildId, RobloxUserId>,
    pub other_accounts: Vec<RobloxUserId>,
    pub flags: UserFlags,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PatreonUser {
    pub user_id: UserId,
    pub kind: PatreonUserType,
    pub patreon_id: i64,
    pub premium_servers: Vec<GuildId>,
    pub transferred_from: Option<UserId>,
    pub transferred_to: Option<UserId>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LinkedUser {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub roblox_id: RobloxUserId,
}

bitflags! {
    #[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct UserFlags: i64 {
        const NONE = 0;
        // Deprecated
        // const PATREON_ALPHA = 1;
        // const PATREON_BETA = 1 << 1;
        const STAFF = 1 << 2;
        const PARTNER = 1 << 3;
    }
}

#[derive(Clone, Copy, Debug, Deserialize_repr, Eq, PartialEq, Serialize_repr)]
#[repr(u32)]
pub enum PatreonUserType {
    None = 0,
    Alpha = 1,
    Beta = 2,
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
        let kind = row.try_get("kind")?;
        let patreon_id = row.try_get("patreon_id")?;
        let premium_servers = row.try_get("premium_servers")?;
        let transferred_from = row.try_get("transferred_from")?;
        let transferred_to = row.try_get("transferred_to")?;

        Ok(Self {
            user_id,
            kind,
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

impl ToSql for PatreonUserType {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        u32::to_sql(&(*self as u32), ty, out)
    }

    fn accepts(ty: &Type) -> bool {
        <u32 as ToSql>::accepts(ty)
    }

    to_sql_checked!();
}

impl<'a> FromSql<'a> for PatreonUserType {
    fn from_sql(
        ty: &Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        match u32::from_sql(ty, raw)? {
            0 => Ok(PatreonUserType::None),
            1 => Ok(PatreonUserType::Alpha),
            2 => Ok(PatreonUserType::Beta),
            _ => unreachable!(),
        }
    }

    fn accepts(ty: &Type) -> bool {
        <String as FromSql>::accepts(ty)
    }
}

impl TryFrom<tokio_postgres::Row> for LinkedUser {
    type Error = tokio_postgres::Error;

    fn try_from(row: tokio_postgres::Row) -> Result<Self, Self::Error> {
        let guild_id = row.try_get("guild_id")?;
        let user_id = row.try_get("user_id")?;
        let roblox_id = row.try_get("roblox_id")?;

        Ok(Self {
            guild_id,
            user_id,
            roblox_id,
        })
    }
}
