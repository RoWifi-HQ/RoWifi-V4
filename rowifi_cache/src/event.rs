use async_trait::async_trait;
use deadpool_redis::redis::{self, AsyncCommands};
use rowifi_models::{
    discord::{
        application::interaction::InteractionType,
        cache::{CachedChannel, CachedGuild, CachedMember, CachedRole},
        gateway::{
            event::Event,
            payload::incoming::{
                ChannelCreate, ChannelDelete, ChannelUpdate, GuildCreate, GuildDelete, GuildUpdate,
                InteractionCreate, MemberAdd, MemberChunk, MemberRemove, MemberUpdate,
                MessageCreate, RoleCreate, RoleDelete, RoleUpdate,
            },
        },
    },
    id::{ChannelId, GuildId, RoleId, UserId},
};

use crate::{
    error::CacheError,
    process::{cache_guild, cache_guild_channel, cache_member, cache_partial_member, cache_role},
    Cache,
};

#[async_trait]
pub trait UpdateCache {
    async fn update(&self, cache: &Cache) -> Result<(), CacheError>;
}

#[async_trait]
impl UpdateCache for Event {
    async fn update(&self, cache: &Cache) -> Result<(), CacheError> {
        use Event::{
            ChannelCreate, ChannelDelete, ChannelUpdate, GuildCreate, GuildDelete, GuildUpdate,
            InteractionCreate, MemberAdd, MemberChunk, MemberRemove, MemberUpdate, MessageCreate,
            RoleCreate, RoleDelete, RoleUpdate,
        };

        match self {
            ChannelCreate(v) => cache.update(&**v).await,
            ChannelDelete(v) => cache.update(&**v).await,
            ChannelUpdate(v) => cache.update(&**v).await,
            GuildCreate(v) => cache.update(&**v).await,
            GuildDelete(v) => cache.update(v).await,
            GuildUpdate(v) => cache.update(&**v).await,
            InteractionCreate(v) => cache.update(&**v).await,
            MemberAdd(v) => cache.update(&**v).await,
            MemberChunk(v) => cache.update(v).await,
            MemberRemove(v) => cache.update(v).await,
            MemberUpdate(v) => cache.update(&**v).await,
            MessageCreate(v) => cache.update(&**v).await,
            RoleCreate(v) => cache.update(v).await,
            RoleUpdate(v) => cache.update(v).await,
            RoleDelete(v) => cache.update(v).await,
            _ => Ok(()),
        }
    }
}

#[async_trait]
impl UpdateCache for ChannelCreate {
    async fn update(&self, c: &Cache) -> Result<(), CacheError> {
        if let Some(guild_id) = self.guild_id {
            let guild_id = GuildId(guild_id);
            if let Some(mut guild) = c.guild(guild_id).await? {
                let mut pipeline = redis::pipe();

                match cache_guild_channel(&mut pipeline, &self.0) {
                    Ok(()) => {
                        guild.channels.insert(ChannelId(self.id));
                        pipeline.set(CachedGuild::key(guild_id), rmp_serde::to_vec(&guild)?);
                    }
                    Err(err) => {
                        tracing::error!(err = ?err);
                    }
                }

                let mut conn = c.get().await?;
                pipeline.query_async(&mut conn).await?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl UpdateCache for ChannelDelete {
    async fn update(&self, c: &Cache) -> Result<(), CacheError> {
        if let Some(guild_id) = self.guild_id {
            let mut pipeline = redis::pipe();

            let guild_id = GuildId(guild_id);
            if let Some(mut guild) = c.guild(guild_id).await? {
                guild.channels.remove(&ChannelId(self.id));
                pipeline.set(CachedGuild::key(guild_id), rmp_serde::to_vec(&guild)?);
            }

            pipeline.del(CachedChannel::key(ChannelId(self.id)));

            let mut conn = c.get().await?;
            pipeline.query_async(&mut conn).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl UpdateCache for ChannelUpdate {
    async fn update(&self, c: &Cache) -> Result<(), CacheError> {
        if self.guild_id.is_some() {
            let mut pipeline = redis::pipe();

            cache_guild_channel(&mut pipeline, self)?;

            let mut conn = c.get().await?;
            pipeline.query_async(&mut conn).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl UpdateCache for GuildCreate {
    async fn update(&self, c: &Cache) -> Result<(), CacheError> {
        let mut pipeline = redis::pipe();
        if let GuildCreate::Available(guild) = self {
            cache_guild(&mut pipeline, guild)?;
        } else {
            return Ok(());
        }

        let mut conn = c.0.pool.get().await?;
        pipeline.query_async(&mut conn).await?;

        Ok(())
    }
}

#[async_trait]
impl UpdateCache for GuildDelete {
    async fn update(&self, c: &Cache) -> Result<(), CacheError> {
        let guild_id = GuildId::new(self.id.get());
        if let Some(guild) = c.guild(guild_id).await? {
            let mut pipeline = redis::pipe();
            pipeline.del(CachedGuild::key(guild_id));

            for channel in guild.channels {
                pipeline.del(CachedChannel::key(channel));
            }
            for role in guild.roles {
                pipeline.del(CachedRole::key(role));
            }

            let mut conn = c.get().await?;
            pipeline.query_async(&mut conn).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl UpdateCache for GuildUpdate {
    async fn update(&self, c: &Cache) -> Result<(), CacheError> {
        let guild_id = GuildId::new(self.id.get());
        if let Some(mut guild) = c.guild(guild_id).await? {
            guild.name.clone_from(&self.name);
            guild.icon = self.icon;
            guild.owner_id = UserId(self.owner_id);

            let mut conn = c.get().await?;
            conn.set(CachedGuild::key(guild_id), rmp_serde::to_vec(&guild)?)
                .await?;
        }
        Ok(())
    }
}

#[async_trait]
impl UpdateCache for MemberAdd {
    async fn update(&self, c: &Cache) -> Result<(), CacheError> {
        let mut pipeline = redis::pipe();

        let guild_id = GuildId(self.guild_id);
        cache_member(&mut pipeline, guild_id, &self.member)?;

        pipeline.sadd(format!("discord:m:{guild_id}"), self.user.id.get());

        let mut conn = c.get().await?;
        pipeline.query_async(&mut conn).await?;

        Ok(())
    }
}

#[async_trait]
impl UpdateCache for MemberChunk {
    async fn update(&self, c: &Cache) -> Result<(), CacheError> {
        if self.members.is_empty() {
            return Ok(());
        }

        let mut pipeline = redis::pipe();
        let guild_id = GuildId(self.guild_id);

        for member in &self.members {
            if let Err(err) = cache_member(&mut pipeline, guild_id, member) {
                tracing::error!(err = ?err);
            }
            pipeline.sadd(format!("discord:m:{guild_id}"), member.user.id.get());
        }

        let mut conn = c.get().await?;
        pipeline.query_async(&mut conn).await?;

        Ok(())
    }
}

#[async_trait]
impl UpdateCache for MemberRemove {
    async fn update(&self, c: &Cache) -> Result<(), CacheError> {
        let mut pipeline = redis::pipe();

        let guild_id = GuildId(self.guild_id);
        let user_id = UserId(self.user.id);

        pipeline.del(CachedMember::key(guild_id, user_id));
        pipeline.srem(format!("discord:m:{guild_id}"), self.user.id.get());

        let mut conn = c.get().await?;
        pipeline.query_async(&mut conn).await?;

        Ok(())
    }
}

#[async_trait]
impl UpdateCache for MemberUpdate {
    async fn update(&self, c: &Cache) -> Result<(), CacheError> {
        let guild_id = GuildId(self.guild_id);
        let user_id = UserId(self.user.id);

        if let Some(mut member) = c.guild_member(guild_id, user_id).await? {
            member.nickname.clone_from(&self.nick);
            member.roles = self.roles.iter().map(|r| RoleId(*r)).collect();

            let mut conn = c.get().await?;
            conn.set(
                CachedMember::key(guild_id, user_id),
                rmp_serde::to_vec(&member)?,
            )
            .await?;
        }

        Ok(())
    }
}

#[async_trait]
impl UpdateCache for MessageCreate {
    async fn update(&self, c: &Cache) -> Result<(), CacheError> {
        if let (Some(guild_id), Some(member)) = (self.guild_id, &self.member) {
            let mut pipeline = redis::pipe();

            let guild_id = GuildId(guild_id);
            cache_partial_member(&mut pipeline, guild_id, member, &self.author)?;

            pipeline.sadd(format!("discord:m:{guild_id}"), self.author.id.get());

            let mut conn = c.get().await?;
            pipeline.query_async(&mut conn).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl UpdateCache for InteractionCreate {
    async fn update(&self, c: &Cache) -> Result<(), CacheError> {
        if let Some(guild_id) = self.guild_id {
            if self.0.kind == InteractionType::ApplicationCommand {
                let mut pipeline = redis::pipe();

                let guild_id = GuildId(guild_id);
                let member = self.0.member.as_ref().unwrap();
                let user = member.user.as_ref().unwrap();
                cache_partial_member(&mut pipeline, guild_id, member, user)?;

                pipeline.sadd(format!("discord:m:{guild_id}"), user.id.get());

                let mut conn = c.get().await?;
                pipeline.query_async(&mut conn).await?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl UpdateCache for RoleCreate {
    async fn update(&self, c: &Cache) -> Result<(), CacheError> {
        let mut pipeline = redis::pipe();

        cache_role(&mut pipeline, &self.role)?;
        let guild_id = GuildId(self.guild_id);
        if let Some(mut guild) = c.guild(guild_id).await? {
            guild.roles.insert(RoleId(self.role.id));
            pipeline.set(CachedGuild::key(guild_id), rmp_serde::to_vec(&guild)?);
        }

        let mut conn = c.get().await?;
        pipeline.query_async(&mut conn).await?;

        Ok(())
    }
}

#[async_trait]
impl UpdateCache for RoleDelete {
    async fn update(&self, c: &Cache) -> Result<(), CacheError> {
        let mut pipeline = redis::pipe();

        let guild_id = GuildId(self.guild_id);
        if let Some(mut guild) = c.guild(guild_id).await? {
            guild.roles.remove(&RoleId(self.role_id));
            pipeline.set(CachedGuild::key(guild_id), rmp_serde::to_vec(&guild)?);
        }

        pipeline.del(CachedRole::key(RoleId(self.role_id)));

        let mut conn = c.get().await?;
        pipeline.query_async(&mut conn).await?;

        Ok(())
    }
}

#[async_trait]
impl UpdateCache for RoleUpdate {
    async fn update(&self, c: &Cache) -> Result<(), CacheError> {
        let mut pipeline = redis::pipe();

        cache_role(&mut pipeline, &self.role)?;

        let guild_id = GuildId(self.guild_id);
        if let Some(mut guild) = c.guild(guild_id).await? {
            guild.roles.insert(RoleId(self.role.id));
            pipeline.set(CachedGuild::key(guild_id), rmp_serde::to_vec(&guild)?);
        }

        let mut conn = c.get().await?;
        pipeline.query_async(&mut conn).await?;

        Ok(())
    }
}
