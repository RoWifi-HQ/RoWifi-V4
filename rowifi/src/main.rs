#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod commands;
mod utils;

use commands::user::user_config;
use deadpool_redis::{Manager as RedisManager, Pool as RedisPool, Runtime};
use rowifi_cache::Cache;
use rowifi_database::Database;
use rowifi_framework::{context::BotContext, Framework};
use rowifi_models::discord::{
    gateway::{
        payload::outgoing::update_presence::UpdatePresencePayload,
        presence::{ActivityType, MinimalActivity, Status},
    },
    id::{marker::ApplicationMarker, Id},
};
use rowifi_roblox::RobloxClient;
use std::{
    error::Error,
    future::{ready, Ready},
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use tokio::task::JoinError;
use tokio_stream::StreamExt;
use tower::Service;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};
use twilight_gateway::{stream::ShardEventStream, Config as GatewayConfig, Event, Intents};
use twilight_http::Client as TwilightClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(EnvFilter::from_default_env()))
        .init();

    let application_id = std::env::var("APPLICATION_ID")
        .expect("expected the application id")
        .parse()
        .unwrap();
    let connection_string =
        std::env::var("DATABASE_CONN").expect("expected a database connection string.");
    let bot_token = std::env::var("BOT_TOKEN").expect("expected the bot token");
    let shard_count = std::env::var("SHARDS_COUNT")
        .expect("expected the shard count")
        .parse()
        .unwrap();
    let redis_url = std::env::var("REDIS_CONN").expect("Expected the redis connection url");
    let open_cloud_auth =
        std::env::var("OPEN_CLOUD_AUTH").expect("Expected the open cloud auth key");

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
    let roblox = RobloxClient::new(&open_cloud_auth);
    let bot_context = BotContext::new(
        Id::<ApplicationMarker>::new(application_id),
        twilight_http,
        database,
        cache,
        roblox,
    );

    let mut framework = Framework::new(bot_context.clone());
    user_config(&mut framework);

    let mut rowifi = RoWifi {
        bot: bot_context,
        framework,
    };

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
            }
            None => break,
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
    pub bot: BotContext,
}

impl Service<(u64, Event)> for RoWifi {
    type Response = ();
    type Error = JoinError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: (u64, Event)) -> Self::Future {
        let fut = self.framework.call(&req.1);
        tokio::spawn(async move {
            let _ = fut.await;
        });

        ready(Ok(()))
    }
}
