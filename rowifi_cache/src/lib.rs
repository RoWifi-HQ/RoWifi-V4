#![deny(clippy::all, clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    dependency_on_unit_never_type_fallback
)]

mod event;
mod process;

pub mod error;

use process::{cache_guild, cache_member, cache_user};
use redis::{aio::ConnectionManager, AsyncCommands, Client as RedisClient};
use rowifi_models::{
    discord::{
        cache::{CachedGuild, CachedMember, CachedRole, CachedUser},
        guild::{Guild, Member},
    },
    id::{GuildId, RoleId, UserId},
};
use std::{collections::HashSet, sync::Arc};

use error::CacheError;
use event::UpdateCache;

pub struct CacheInner {
    pub(crate) conn: ConnectionManager,
}

#[derive(Clone)]
pub struct Cache(Arc<CacheInner>);

impl Cache {
    /// Create a Cache client given a Redis client.
    ///
    /// # Errors
    ///
    /// See [`CacheError`] for details.
    pub async fn new(client: RedisClient) -> Result<Self, CacheError> {
        let conn = client.get_connection_manager().await?;
        Ok(Self(Arc::new(CacheInner { conn })))
    }

    /// Update data in the cache.
    ///
    /// # Errors
    ///
    /// See [`CacheError`] for details.
    pub async fn update<T: UpdateCache>(&self, value: &T) -> Result<(), CacheError> {
        value.update(self).await
    }

    /// Returns a connection from the pool.
    ///
    /// # Errors
    ///
    /// See [`CacheError`] for details.
    #[must_use]
    pub fn get(&self) -> ConnectionManager {
        self.0.conn.clone()
    }

    /// Returns a user from the cache.
    ///
    /// # Errors
    ///
    /// See [`CacheError`] for details.
    pub async fn user(&self, user_id: UserId) -> Result<Option<CachedUser>, CacheError> {
        let mut conn = self.get();
        let res: Option<Vec<u8>> = conn.get(CachedUser::key(user_id)).await?;

        if let Some(res) = res {
            Ok(rmp_serde::from_slice(&res)?)
        } else {
            Ok(None)
        }
    }

    /// Returns the server from the cache.
    ///
    /// # Errors
    ///
    /// See [`CacheError`] for details.
    pub async fn guild(&self, id: GuildId) -> Result<Option<CachedGuild>, CacheError> {
        let mut conn = self.get();
        let res: Option<Vec<u8>> = conn.get(CachedGuild::key(id)).await?;

        if let Some(res) = res {
            Ok(rmp_serde::from_slice(&res)?)
        } else {
            Ok(None)
        }
    }

    /// Returns a member for a particular guild from the cache.
    ///
    /// # Errors
    ///
    /// See [`CacheError`] for details.
    pub async fn guild_member(
        &self,
        guild_id: GuildId,
        user_id: UserId,
    ) -> Result<Option<CachedMember>, CacheError> {
        let mut conn = self.get();
        let res: Option<Vec<u8>> = conn.get(CachedMember::key(guild_id, user_id)).await?;

        if let Some(res) = res {
            Ok(rmp_serde::from_slice(&res)?)
        } else {
            Ok(None)
        }
    }

    /// Returns a list of members of a particular guild from the cache.
    ///
    /// # Errors
    ///
    /// See [`CacheError`] for details.
    pub async fn guild_members(
        &self,
        guild_id: GuildId,
        user_ids: impl Iterator<Item = UserId>,
    ) -> Result<Vec<CachedMember>, CacheError> {
        let mut conn = self.get();
        let keys = user_ids
            .into_iter()
            .map(|u| CachedMember::key(guild_id, u))
            .collect::<Vec<_>>();

        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let res: Vec<Vec<u8>> = conn.get(keys).await?;
        let mut members = Vec::new();
        for r in res {
            if let Ok(member) = rmp_serde::from_slice::<CachedMember>(&r) {
                members.push(member);
            }
        }

        Ok(members)
    }

    /// Returns all the cached members for a particular guild.
    ///
    /// # Errors
    ///
    /// See [`CacheError`] for details.
    pub async fn guild_members_set(&self, id: GuildId) -> Result<HashSet<UserId>, CacheError> {
        let mut conn = self.get();
        let res: Vec<u64> = conn.smembers(format!("discord:m:{id}")).await?;

        Ok(res.into_iter().map(UserId::new).collect())
    }

    /// Returns the roles of the server.
    ///
    /// # Errors
    ///
    /// See [`CacheError`] for details.
    pub async fn guild_roles(
        &self,
        role_ids: impl Iterator<Item = RoleId>,
    ) -> Result<Vec<CachedRole>, CacheError> {
        let mut conn = self.get();
        let keys = role_ids
            .into_iter()
            .map(CachedRole::key)
            .collect::<Vec<_>>();
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        let res: Vec<Vec<u8>> = conn.get(keys).await?;

        let mut roles = Vec::new();
        for r in res {
            if let Ok(role) = rmp_serde::from_slice::<CachedRole>(&r) {
                roles.push(role);
            }
        }

        Ok(roles)
    }

    /// Returns a list of users from the cache.
    ///
    /// # Errors
    ///
    /// See [`CacheError`] for details.
    pub async fn users(
        &self,
        user_ids: impl Iterator<Item = UserId>,
    ) -> Result<Vec<CachedUser>, CacheError> {
        let mut conn = self.get();
        let keys = user_ids
            .into_iter()
            .map(CachedUser::key)
            .collect::<Vec<_>>();

        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let res: Vec<Vec<u8>> = conn.get(keys).await?;
        let mut users = Vec::new();
        for r in res {
            if let Ok(user) = rmp_serde::from_slice::<CachedUser>(&r) {
                users.push(user);
            }
        }

        Ok(users)
    }

    /// Add a member to the cache. Replaces if the member already exists.
    ///
    /// # Errors
    ///
    /// See [`CacheError`] for details.
    pub async fn cache_member(
        &self,
        guild_id: GuildId,
        member: &Member,
    ) -> Result<(CachedMember, CachedUser), CacheError> {
        let mut pipeline = redis::pipe();
        let cached_member = cache_member(&mut pipeline, guild_id, member)?;
        let cached_user = cache_user(&mut pipeline, &member.user)?;

        let mut conn = self.get();
        pipeline.query_async(&mut conn).await?;

        Ok((cached_member, cached_user))
    }

    /// Add a guild to the cache. Replaces if the guild already exists.
    ///
    /// # Errors
    ///
    /// See [`CacheError`] for details.
    pub async fn cache_guild(&self, guild: Guild) -> Result<CachedGuild, CacheError> {
        let mut pipeline = redis::pipe();
        let cached = cache_guild(&mut pipeline, &guild)?;

        let mut conn = self.get();
        pipeline.query_async(&mut conn).await?;

        Ok(cached)
    }
}
