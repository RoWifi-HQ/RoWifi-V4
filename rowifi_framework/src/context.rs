use rowifi_cache::Cache;
use rowifi_database::Database;
use rowifi_models::{
    discord::{
        application::interaction::application_command::CommandInteractionDataResolved,
        cache::CachedMember,
        id::{marker::InteractionMarker, Id},
    },
    id::{ChannelId, GuildId, UserId}, guild::PartialRoGuild,
};
use std::{ops::Deref, sync::Arc};
use twilight_http::{Client as TwilightClient, error::ErrorType as DiscordErrorType};

use crate::error::FrameworkError;

pub struct BotContextInner {
    /// The module used to make requests to discord
    pub http: Arc<TwilightClient>,
    pub database: Arc<Database>,
    /// The cache holding all discord data
    pub cache: Cache,
}

#[derive(Clone)]
pub struct BotContext(Arc<BotContextInner>);

pub struct CommandContext {
    pub bot: BotContext,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub author_id: UserId,
    pub interaction_id: Id<InteractionMarker>,
    pub interaction_token: String,
    pub resolved: CommandInteractionDataResolved,
}

impl BotContext {
    pub fn new(http: Arc<TwilightClient>, database: Arc<Database>, cache: Cache) -> Self {
        Self(Arc::new(BotContextInner {
            http,
            database,
            cache,
        }))
    }

    pub async fn member(
        &self,
        guild_id: GuildId,
        user_id: UserId,
    ) -> Result<Option<CachedMember>, FrameworkError> {
        if let Some(member) = self.cache.guild_member(guild_id, user_id).await? {
            return Ok(Some(member));
        }
        let res = self.http.guild_member(guild_id.0, user_id.0).await;
        match res {
            Err(e) => {
                if let DiscordErrorType::Response {
                    body: _,
                    error: _,
                    status,
                } = e.kind()
                {
                    if *status == 404 {
                        return Ok(None);
                    }
                }
                Err(e.into())
            }
            Ok(res) => {
                let member = res.model().await?;
                let cached = self.cache.cache_member(guild_id, &member).await?;
                Ok(Some(cached))
            }
        }
    }

    pub async fn get_guild(&self, statement: &str, guild_id: GuildId) -> Result<PartialRoGuild, FrameworkError> {
        let res = self.database.query_opt::<PartialRoGuild>(statement, &[&guild_id]).await?;
        if let Some(guild) = res {
            Ok(guild)
        } else {
            self.database.execute("INSERT INTO guilds(guild_id) VALUES($1)", &[&guild_id]).await?;
            Ok(PartialRoGuild::new(guild_id))
        }
    }
}

impl Deref for BotContext {
    type Target = BotContextInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
