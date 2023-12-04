mod event;
mod process;

pub mod error;

use deadpool_redis::{redis::AsyncCommands, Connection, Pool as RedisPool, PoolError};
use rowifi_models::{
    discord::cache::{CachedGuild, CachedMember},
    id::{GuildId, UserId},
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
    pub fn new(pool: RedisPool) -> Self {
        Self(Arc::new(CacheInner { pool }))
    }

    pub async fn update<T: UpdateCache>(&self, value: &T) -> Result<(), CacheError> {
        value.update(self).await
    }

    pub async fn get(&self) -> Result<Connection, PoolError> {
        self.0.pool.get().await
    }

    pub async fn guild(&self, id: GuildId) -> Result<Option<CachedGuild>, CacheError> {
        let mut conn = self.get().await?;
        let res: Option<Vec<u8>> = conn.get(CachedGuild::key(id)).await?;

        if let Some(res) = res {
            Ok(rmp_serde::from_slice(&res)?)
        } else {
            Ok(None)
        }
    }

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
}
