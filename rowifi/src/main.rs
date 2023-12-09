use rowifi_cache::Cache;
use rowifi_database::Database;
use rowifi_framework::{context::BotContext, Framework};
use rowifi_models::discord::gateway::{
    payload::outgoing::update_presence::UpdatePresencePayload,
    presence::{ActivityType, MinimalActivity, Status},
};
use tokio::task::JoinError;
use tower::Service;
use std::{error::Error, sync::Arc, time::Duration, future::{Ready, ready}, task::{Poll, Context}};
use twilight_gateway::{Config as GatewayConfig, Intents, stream::ShardEventStream, Event};
use twilight_http::Client as TwilightClient;
use deadpool_redis::{Pool as RedisPool, Manager as RedisManager, Runtime};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let connection_string =
        std::env::var("DATABASE_CONN").expect("expected a database connection string.");
    let bot_token = std::env::var("BOT_TOKEN").expect("expected the bot token");
    let shard_count = std::env::var("SHARDS_COUNT").expect("expected the shard count").parse().unwrap();
    let redis_url = std::env::var("REDIS_CONN").expect("Expected the redis connection url");

    let redis = RedisPool::builder(RedisManager::new(redis_url).unwrap())
        .max_size(16)
        .runtime(Runtime::Tokio1)
        .recycle_timeout(Some(Duration::from_secs(30)))
        .wait_timeout(Some(Duration::from_secs(30)))
        .create_timeout(Some(Duration::from_secs(30)))
        .build()
        .unwrap();

    let cache = Cache::new(redis);
    let database = Arc::new(Database::new(&connection_string).await);
    let twilight_http = Arc::new(TwilightClient::new(bot_token.clone()));
    let bot_context = BotContext::new(twilight_http, database, cache);
    let framework = Framework::new(bot_context.clone());
    let mut rowifi = RoWifi { bot: bot_context, framework };

    let activity = MinimalActivity {
        kind: ActivityType::Playing,
        name: "rowifi.xyz".into(),
        url: None,
    }
    .into();

    let shards_config = GatewayConfig::builder(
        bot_token,
        Intents::GUILDS | Intents::GUILD_MESSAGES | Intents::GUILD_MEMBERS,
    )
    .presence(UpdatePresencePayload {
        activities: vec![activity],
        afk: false,
        since: None,
        status: Status::Online,
    })
    .build();
    let mut shards = twilight_gateway::stream::create_range(
        0..shard_count,
        shard_count,
        shards_config,
        |_, builder| builder.build(),
    )
    .collect::<Vec<_>>();


    let mut stream = ShardEventStream::new(shards.iter_mut());
    loop {
        let (shard, event) = match stream.next().await {
            Some((shard, Ok(event))) => (shard, event),
            Some((_, Err(source))) => {
                tracing::warn!("error receiving event {}", source);

                if source.is_fatal() {
                    break;
                }

                continue;
            },
            None => break
        };
        if let Err(err) = rowifi.bot.cache.update(&event).await {
            tracing::error!(err = ?err, "cache error: ");
        }
        let _ = rowifi.call((shard.id().number(), event)).await;
    }

    Ok(())
}

pub struct RoWifi {
    pub framework: Framework,
    pub bot: BotContext
}

impl Service<(u64, Event)> for RoWifi {
    type Response = ();
    type Error = JoinError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: (u64, Event)) -> Self::Future {
        ready(Ok(()))
    }
}
