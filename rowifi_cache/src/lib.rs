#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod event;
mod process;

pub mod error;

use deadpool_redis::{redis::AsyncCommands, Connection, Pool as RedisPool, PoolError};
use rowifi_models::{
    discord::{
        cache::{CachedGuild, CachedMember},
        guild::Member,
    },
    id::{GuildId, RoleId, UserId},
};
use std::sync::Arc;

use error::CacheError;
use event::UpdateCache;

pub struct CacheInner {
    pub(crate) pool: RedisPool,
}

#[derive(Clone)]
pub struct Cache(Arc<CacheInner>);

impl Cache {
    #[must_use]
    pub fn new(pool: RedisPool) -> Self {
        Self(Arc::new(CacheInner { pool }))
    }

    /// Update data in the cache.
    ///
    /// # Errors
    ///
    /// Returns Err if the data could not be updated.
    pub async fn update<T: UpdateCache>(&self, value: &T) -> Result<(), CacheError> {
        value.update(self).await
    }

    /// Returns a connection from the pool.
    ///
    /// # Errors
    ///
    /// Returns Err if there are no more connections left or the connection fails.
    pub async fn get(&self) -> Result<Connection, PoolError> {
        self.0.pool.get().await
    }

    /// Returns the server from the cache.
    ///
    /// # Errors
    ///
    /// Returns Err if the connection to redis fails or the deserialization fails.
    pub async fn guild(&self, id: GuildId) -> Result<Option<CachedGuild>, CacheError> {
        let mut conn = self.get().await?;
        let res: Option<Vec<u8>> = conn.get(CachedGuild::key(id)).await?;

        if let Some(res) = res {
            Ok(rmp_serde::from_slice(&res)?)
        } else {
            Ok(None)
        }
    }

    /// Returns a member from the cache.
    ///
    /// # Errors
    ///
    /// Returns Err if the connection to redis fails or the deserialization fails.
    pub async fn guild_member(
        &self,
        guild_id: GuildId,
        user_id: UserId,
    ) -> Result<Option<CachedMember>, CacheError> {
        let mut conn = self.get().await?;
        let res: Option<Vec<u8>> = conn.get(CachedMember::key(guild_id, user_id)).await?;

        if let Some(res) = res {
            Ok(rmp_serde::from_slice(&res)?)
        } else {
            Ok(None)
        }
    }

    /// Add a member to the cache. Replaces if the member already exists.
    ///
    /// # Errors
    ///
    /// Returns Err if the connection to redis fails or the serialization fails.
    pub async fn cache_member(
        &self,
        guild_id: GuildId,
        member: &Member,
    ) -> Result<CachedMember, CacheError> {
        let cached = CachedMember {
            id: UserId(member.user.id),
            roles: member.roles.iter().map(|r| RoleId(*r)).collect(),
            nickname: member.nick.clone(),
            username: member.user.name.clone(),
            discriminator: member.user.discriminator,
        };

        let mut conn = self.get().await?;
        conn.set(
            CachedMember::key(guild_id, cached.id),
            rmp_serde::to_vec(&cached)?,
        )
        .await?;

        Ok(cached)
    }
}
