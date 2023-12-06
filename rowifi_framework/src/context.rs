use rowifi_cache::Cache;
use rowifi_database::Database;
use std::sync::Arc;
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

impl BotContext {
    pub fn new(http: Arc<TwilightClient>, database: Arc<Database>, cache: Cache) -> Self {
        Self(Arc::new(BotContextInner { http, database, cache }))
    }
}
