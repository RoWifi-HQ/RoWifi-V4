use rowifi_cache::Cache;
use rowifi_core::error::RoError;
use rowifi_database::Database;
use rowifi_models::{
    discord::{
        application::interaction::InteractionDataResolved,
        cache::{CachedGuild, CachedMember},
        channel::{
            message::{Component, Embed, MessageFlags},
            Message,
        },
        http::{
            attachment::Attachment,
            interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
        },
        id::{
            marker::{ApplicationMarker, InteractionMarker},
            Id,
        },
    },
    guild::PartialRoGuild,
    id::{ChannelId, GuildId, UserId},
};
use rowifi_roblox::RobloxClient;
use std::{
    future::IntoFuture,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use twilight_http::{
    error::ErrorType as DiscordErrorType, response::ResponseFuture, Client as TwilightClient,
};
use twilight_validate::message::{
    components as _components, content as _content, embeds as _embeds, MessageValidationError,
};

pub struct BotContextInner {
    pub application_id: Id<ApplicationMarker>,
    /// The module used to make requests to discord
    pub http: Arc<TwilightClient>,
    pub database: Arc<Database>,
    /// The cache holding all discord data
    pub cache: Cache,
    pub roblox: RobloxClient,
}

#[derive(Clone)]
pub struct BotContext(Arc<BotContextInner>);

pub struct CommandContext {
    pub name: String,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub author_id: UserId,
    pub interaction_id: Id<InteractionMarker>,
    pub interaction_token: String,
    pub resolved: Option<InteractionDataResolved>,
    pub callback_invoked: AtomicBool,
}

pub enum DeferredResponse {
    Ephemeral,
    Normal,
}

impl BotContext {
    #[must_use]
    pub fn new(
        application_id: Id<ApplicationMarker>,
        http: Arc<TwilightClient>,
        database: Arc<Database>,
        cache: Cache,
        roblox: RobloxClient,
    ) -> Self {
        Self(Arc::new(BotContextInner {
            application_id,
            http,
            database,
            cache,
            roblox,
        }))
    }

    /// Finds the member in the cache or requests it through the http module.
    ///
    /// # Errors
    ///
    /// Return Err if the request to the cache or http fails or if the member
    /// doe not exist.
    pub async fn member(
        &self,
        guild_id: GuildId,
        user_id: UserId,
    ) -> Result<Option<CachedMember>, RoError> {
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

    /// Gets the guild from the database. If it does not exist, it creates a row
    /// in the database and returns it.
    ///
    /// # Errors
    ///
    /// See [`DatabaseError`](rowifi_database::DatabaseError) for details
    pub async fn get_guild(
        &self,
        statement: &str,
        guild_id: GuildId,
    ) -> Result<PartialRoGuild, RoError> {
        let res = self
            .database
            .query_opt::<PartialRoGuild>(statement, &[&guild_id])
            .await?;
        if let Some(guild) = res {
            Ok(guild)
        } else {
            self.database
                .execute("INSERT INTO guilds(guild_id) VALUES($1)", &[&guild_id])
                .await?;
            Ok(PartialRoGuild::new(guild_id))
        }
    }

    /// Get the server from the cache. If it is not present in the cache, get it from the Discord API.
    ///
    /// # Errors
    ///
    /// Will return an error on a cache or discord error. See [`RoError`] for details.
    pub async fn server(&self, guild_id: GuildId) -> Result<CachedGuild, RoError> {
        if let Ok(Some(guild)) = self.cache.guild(guild_id).await {
            Ok(guild)
        } else {
            let guild = self.http.guild(guild_id.0).await?.model().await?;
            let cached = self.cache.cache_guild(guild).await?;
            Ok(cached)
        }
    }
}

impl Deref for BotContext {
    type Target = BotContextInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl CommandContext {
    pub fn respond<'a>(&'a self, bot: &'a BotContext) -> Responder<'a> {
        Responder::new(self, bot)
    }

    /// Sends an interaction response saying the response will be deferred.
    ///
    /// # Errors
    ///
    /// See [`TwilightError`](twilight_http::Error) for details.
    pub async fn defer_response(
        &self,
        bot: &BotContext,
        defer: DeferredResponse,
    ) -> Result<(), RoError> {
        let data = match defer {
            DeferredResponse::Ephemeral => InteractionResponseData {
                flags: Some(MessageFlags::EPHEMERAL),
                ..Default::default()
            },
            DeferredResponse::Normal => InteractionResponseData::default(),
        };

        bot.http
            .interaction(bot.application_id)
            .create_response(
                self.interaction_id,
                &self.interaction_token,
                &InteractionResponse {
                    kind: InteractionResponseType::DeferredChannelMessageWithSource,
                    data: Some(data),
                },
            )
            .await?;
        Ok(())
    }
}

pub struct Responder<'a> {
    ctx: &'a CommandContext,
    bot: &'a BotContext,
    content: Option<&'a str>,
    components: Option<&'a [Component]>,
    embeds: Option<&'a [Embed]>,
    files: Option<&'a [Attachment]>,
    flags: Option<MessageFlags>,
}

impl<'a> Responder<'a> {
    pub fn new(ctx: &'a CommandContext, bot: &'a BotContext) -> Self {
        Self {
            ctx,
            bot,
            content: None,
            components: None,
            embeds: None,
            files: None,
            flags: None,
        }
    }

    /// Sets the content of the response.
    ///
    /// # Errors
    ///
    /// See [`MessageValidationError`] for details.
    pub fn content(mut self, content: &'a str) -> Result<Self, MessageValidationError> {
        _content(content)?;

        self.content = Some(content);
        Ok(self)
    }

    /// Sets the components of the response.
    ///
    /// # Errors
    ///
    /// See [`MessageValidationError`] for details.
    pub fn components(
        mut self,
        components: &'a [Component],
    ) -> Result<Self, MessageValidationError> {
        _components(components)?;

        self.components = Some(components);
        Ok(self)
    }

    /// Sets the embeds of the response.
    ///
    /// # Errors
    ///
    /// See [`MessageValidationError`] for details.
    pub fn embeds(mut self, embeds: &'a [Embed]) -> Result<Self, MessageValidationError> {
        _embeds(embeds)?;

        self.embeds = Some(embeds);
        Ok(self)
    }

    /// Sets the files sent in the response.
    ///
    /// # Errors
    ///
    /// See [`MessageValidationError`] for details.
    #[must_use]
    pub fn files(mut self, files: &'a [Attachment]) -> Self {
        self.files = Some(files);
        self
    }

    /// Sets the response message flags.
    ///
    /// # Errors
    ///
    /// See [`MessageValidationError`] for details.
    #[must_use]
    pub fn flags(mut self, flags: MessageFlags) -> Self {
        self.flags = Some(flags);
        self
    }
}

impl IntoFuture for Responder<'_> {
    type IntoFuture = ResponseFuture<Message>;
    type Output = Result<twilight_http::Response<Message>, twilight_http::Error>;

    fn into_future(self) -> Self::IntoFuture {
        if self.ctx.callback_invoked.load(Ordering::Relaxed) {
            let client = self.bot.http.interaction(self.bot.application_id);
            let mut req = client.create_followup(&self.ctx.interaction_token);
            if let Some(content) = self.content {
                req = req.content(content);
            }
            if let Some(components) = self.components {
                req = req.components(components);
            }
            if let Some(embeds) = self.embeds {
                req = req.embeds(embeds);
            }
            if let Some(files) = self.files {
                req = req.attachments(files);
            }
            if let Some(flags) = self.flags {
                req = req.flags(flags);
            }
            req.into_future()
        } else {
            let client = self.bot.http.interaction(self.bot.application_id);
            let req = client
                .update_response(&self.ctx.interaction_token)
                .content(self.content)
                .components(self.components)
                .embeds(self.embeds)
                .attachments(self.files.unwrap_or_default());
            self.ctx.callback_invoked.store(true, Ordering::Relaxed);
            req.into_future()
        }
    }
}
