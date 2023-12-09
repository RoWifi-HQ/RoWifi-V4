use rowifi_cache::Cache;
use rowifi_database::Database;
use rowifi_models::{id::{GuildId, ChannelId}, discord::{guild::PartialMember, id::{Id, marker::InteractionMarker}, application::interaction::application_command::CommandInteractionDataResolved}};
use std::{sync::Arc, ops::Deref};
use twilight_http::Client as TwilightClient;

pub struct BotContextInner {
    /// The module used to make requests to discord
    pub http: Arc<TwilightClient>,
    pub database: Arc<Database>,
    /// The cache holding all discord data
    pub cache: Cache
}

#[derive(Clone)]
pub struct BotContext(Arc<BotContextInner>);

pub struct CommandContext {
    pub bot: BotContext,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub member: PartialMember,
    pub interaction_id: Id<InteractionMarker>,
    pub interaction_token: String,
    pub resolved: CommandInteractionDataResolved,
}

impl BotContext {
    pub fn new(http: Arc<TwilightClient>, database: Arc<Database>, cache: Cache) -> Self {
        Self(Arc::new(BotContextInner { http, database, cache }))
    }
}

impl Deref for BotContext {
    type Target = BotContextInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
