use rowifi_cache::Cache;
use rowifi_database::Database;
use rowifi_models::{
    discord::{
        application::interaction::application_command::CommandInteractionDataResolved,
        cache::CachedMember,
        channel::{
            message::{Component, Embed, MessageFlags},
            Message,
        },
        http::{attachment::Attachment, interaction::{InteractionResponse, InteractionResponseType, InteractionResponseData}},
        id::{marker::{InteractionMarker, ApplicationMarker}, Id},
    },
    guild::PartialRoGuild,
    id::{ChannelId, GuildId, UserId},
};
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

use crate::error::FrameworkError;

pub struct BotContextInner {
    pub application_id: Id<ApplicationMarker>,
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
    pub resolved: Option<CommandInteractionDataResolved>,
    pub callback_invoked: AtomicBool,
}

pub enum DeferredResponse {
    Ephemeral,
    Normal
}

impl BotContext {
    pub fn new(application_id: Id<ApplicationMarker>, http: Arc<TwilightClient>, database: Arc<Database>, cache: Cache) -> Self {
        Self(Arc::new(BotContextInner {
            application_id,
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

    pub async fn get_guild(
        &self,
        statement: &str,
        guild_id: GuildId,
    ) -> Result<PartialRoGuild, FrameworkError> {
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
}

impl Deref for BotContext {
    type Target = BotContextInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl CommandContext {
    pub fn respond(&self) -> Responder<'_> {
        Responder::new(self)
    }

    pub async fn defer_response(&self, defer: DeferredResponse) -> Result<(), FrameworkError> {
        let data = match defer {
            DeferredResponse::Ephemeral => InteractionResponseData {
                flags: Some(MessageFlags::EPHEMERAL),
                ..Default::default()
            },
            DeferredResponse::Normal => InteractionResponseData::default()
        };

        self.bot.http.interaction(self.bot.application_id)
            .create_response(self.interaction_id, &self.interaction_token, &InteractionResponse {
                kind: InteractionResponseType::DeferredChannelMessageWithSource,
                data: Some(data)
            })
            .await?;
        Ok(())
    }
}

pub struct Responder<'a> {
    ctx: &'a CommandContext,
    content: Option<&'a str>,
    components: Option<&'a [Component]>,
    embeds: Option<&'a [Embed]>,
    files: Option<&'a [Attachment]>,
    flags: Option<MessageFlags>,
}

impl<'a> Responder<'a> {
    pub fn new(ctx: &'a CommandContext) -> Self {
        Self {
            ctx,
            content: None,
            components: None,
            embeds: None,
            files: None,
            flags: None,
        }
    }

    pub fn content(mut self, content: &'a str) -> Result<Self, MessageValidationError> {
        _content(content)?;

        self.content = Some(content);
        Ok(self)
    }

    pub fn components(
        mut self,
        components: &'a [Component],
    ) -> Result<Self, MessageValidationError> {
        _components(components)?;

        self.components = Some(components);
        Ok(self)
    }

    pub fn embeds(mut self, embeds: &'a [Embed]) -> Result<Self, MessageValidationError> {
        _embeds(embeds)?;

        self.embeds = Some(embeds);
        Ok(self)
    }

    #[must_use]
    pub fn files(mut self, files: &'a [Attachment]) -> Self {
        self.files = Some(files);
        self
    }

    #[must_use]
    pub fn flags(mut self, flags: MessageFlags) -> Self {
        self.flags = Some(flags);
        self
    }

    pub fn exec(self) -> ResponseFuture<Message> {
        if self.ctx.callback_invoked.load(Ordering::Relaxed) {
            let client = self.ctx.bot.http.interaction(self.ctx.bot.application_id);
            let mut req = client.create_followup(&self.ctx.interaction_token);
            if let Some(content) = self.content {
                req = req.content(content).unwrap();
            }
            if let Some(components) = self.components {
                req = req.components(components).unwrap();
            }
            if let Some(embeds) = self.embeds {
                req = req.embeds(embeds).unwrap();
            }
            if let Some(files) = self.files {
                req = req.attachments(files).unwrap();
            }
            if let Some(flags) = self.flags {
                req = req.flags(flags);
            }
            req.into_future()
        } else {
            let client = self.ctx.bot.http.interaction(self.ctx.bot.application_id);
            let req = client
                .update_response(&self.ctx.interaction_token)
                .content(self.content)
                .unwrap()
                .components(self.components)
                .unwrap()
                .embeds(self.embeds)
                .unwrap()
                .attachments(self.files.unwrap_or_default())
                .unwrap();
            self.ctx.callback_invoked.store(true, Ordering::Relaxed);
            req.into_future()
        }
    }
}
