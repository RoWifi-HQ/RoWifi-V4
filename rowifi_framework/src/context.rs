use std::sync::Arc;
use twilight_http::Client as TwilightClient;

pub struct BotContextInner {
    /// The module used to make requests to discord
    pub http: Arc<TwilightClient>,
    
}