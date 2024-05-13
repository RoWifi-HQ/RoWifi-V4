#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod commands;
mod utils;

use axum::{
    body::Body,
    extract::Request,
    http::{StatusCode, Uri},
    middleware::map_request,
    response::Response,
    routing::post,
    Extension, Json, Router, ServiceExt,
};
use deadpool_redis::{Manager as RedisManager, Pool as RedisPool, Runtime};
use ed25519_dalek::{Verifier, VerifyingKey, PUBLIC_KEY_LENGTH};
use hex::FromHex;
use rowifi_cache::Cache;
use rowifi_database::Database;
use rowifi_framework::context::BotContext;
use rowifi_models::discord::{
    application::{
        command::CommandOptionType,
        interaction::{Interaction, InteractionData, InteractionType},
    },
    gateway::{event::Event, payload::incoming::InteractionCreate},
    http::interaction::{InteractionResponse, InteractionResponseType},
    id::{marker::ApplicationMarker, Id},
};
use rowifi_roblox::RobloxClient;
use std::{error::Error, future::Future, pin::Pin, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower::Layer as _;
use tower_http::{
    auth::{AsyncAuthorizeRequest, AsyncRequireAuthorizationLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};
use twilight_http::Client as TwilightClient;
use twilight_standby::Standby;

use crate::commands::{
    assetbinds::{delete_assetbind, new_assetbind, view_assetbinds},
    denylists::{add_group_denylist, add_user_denylist, delete_denylist, view_denylists},
    groupbinds::{delete_groupbind, new_groupbind, view_groupbinds},
    rankbinds::{delete_rankbind, new_rankbind, view_rankbinds},
    user::{
        account_default, account_delete, account_switch, account_view, update_route, userinfo,
        verify_route,
    },
};

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
    let redis_url = std::env::var("REDIS_CONN").expect("Expected the redis connection url");
    let open_cloud_auth =
        std::env::var("OPEN_CLOUD_AUTH").expect("Expected the open cloud auth key");
    let discord_public_key =
        std::env::var("DISCORD_PUBLIC_KEY").expect("Expected the discord public key");

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

    let verifying_key = VerifyingKey::from_bytes(
        &<[u8; PUBLIC_KEY_LENGTH] as FromHex>::from_hex(discord_public_key).unwrap(),
    )
    .unwrap();
    let standby = Standby::new();

    let middleware = map_request(rewrite_request_uri);
    #[allow(unused_mut)]
    let mut router = Router::new()
        .route("/", post(pong))
        .route("/update", post(update_route))
        .route("/rankbinds/new", post(new_rankbind))
        .route("/rankbinds/delete", post(delete_rankbind))
        .route("/rankbinds/view", post(view_rankbinds))
        .route("/groupbinds/new", post(new_groupbind))
        .route("/groupbinds/delete", post(delete_groupbind))
        .route("/groupbinds/view", post(view_groupbinds))
        .route("/assetbinds/new", post(new_assetbind))
        .route("/assetbinds/delete", post(delete_assetbind))
        .route("/assetbinds/view", post(view_assetbinds))
        .route("/denylists/user", post(add_user_denylist))
        .route("/denylists/group", post(add_group_denylist))
        .route("/denylists/delete", post(delete_denylist))
        .route("/denylists/view", post(view_denylists))
        .route("/account/view", post(account_view))
        .route("/account/default", post(account_default))
        .route("/account/switch", post(account_switch))
        .route("/account/delete", post(account_delete))
        .route("/verify", post(verify_route))
        .route("/userinfo", post(userinfo))
        .route("/standby", post(standby_route));

    #[cfg(feature = "tower")]
    {
        router = router.route("/setrank", post(rowifi_tower::set_rank));
    }

    let app = router
        .layer(Extension(Arc::new(standby)))
        .layer(AsyncRequireAuthorizationLayer::new(WebhookAuth))
        .layer(Extension(Arc::new(verifying_key)))
        .layer(Extension(bot_context))
        .layer(TraceLayer::new_for_http());
    let app_with_middleware = middleware.layer(app);
    let listener = TcpListener::bind("0.0.0.0:8000").await?;
    axum::serve(listener, app_with_middleware.into_make_service()).await?;

    Ok(())
}

async fn pong() -> Json<InteractionResponse> {
    Json(InteractionResponse {
        kind: InteractionResponseType::Pong,
        data: None,
    })
}

async fn standby_route(
    bot_standby: Extension<Arc<Standby>>,
    interaction: Json<Interaction>,
) -> Json<InteractionResponse> {
    let _ = bot_standby.process(&Event::InteractionCreate(Box::new(InteractionCreate(
        interaction.0,
    ))));

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredUpdateMessage,
        data: None,
    })
}

async fn rewrite_request_uri(req: Request) -> Request {
    let (mut parts, body) = req.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    let interaction = serde_json::from_slice::<Interaction>(&bytes).unwrap();

    match interaction.kind {
        InteractionType::ApplicationCommand => {
            let Some(InteractionData::ApplicationCommand(data)) = &interaction.data else {
                unreachable!()
            };
            let subcommand_name = if let Some(option) = data.options.first() {
                if option.value.kind() == CommandOptionType::SubCommand
                    || option.value.kind() == CommandOptionType::SubCommandGroup
                {
                    Some(&option.name)
                } else {
                    None
                }
            } else {
                None
            };
            let command_name = if let Some(subcommand_name) = subcommand_name {
                format!("/{}/{subcommand_name}", data.name)
            } else {
                format!("/{}", data.name)
            };
            let mut uri_parts = parts.uri.into_parts();
            uri_parts.path_and_query = Some(command_name.parse().unwrap());
            let new_uri = Uri::from_parts(uri_parts).unwrap();
            parts.uri = new_uri;
        }
        InteractionType::MessageComponent => {
            let mut uri_parts = parts.uri.into_parts();
            uri_parts.path_and_query = Some("/standby".parse().unwrap());
            let new_uri = Uri::from_parts(uri_parts).unwrap();
            parts.uri = new_uri;
        }
        _ => {}
    }

    let body = Body::from(bytes);
    Request::from_parts(parts, body)
}

#[derive(Clone)]
struct WebhookAuth;

impl AsyncAuthorizeRequest<Body> for WebhookAuth {
    type RequestBody = Body;
    type ResponseBody = Body;
    type Future =
        Pin<Box<dyn Future<Output = Result<Request<Body>, Response<Self::ResponseBody>>> + Send>>;

    fn authorize(&mut self, request: Request) -> Self::Future {
        Box::pin(async move {
            let verifying_key = request
                .extensions()
                .get::<Arc<VerifyingKey>>()
                .unwrap()
                .clone();

            let (parts, body) = request.into_parts();
            let Some(timestamp) = parts.headers.get("x-signature-timestamp") else {
                return Err(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::empty())
                    .unwrap());
            };
            let signature = match parts
                .headers
                .get("x-signature-ed25519")
                .and_then(|v| v.to_str().ok())
            {
                Some(h) => h.parse().unwrap(),
                None => {
                    return Err(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::empty())
                        .unwrap());
                }
            };

            let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
            if verifying_key
                .verify([timestamp.as_bytes(), &bytes].concat().as_ref(), &signature)
                .is_err()
            {
                return Err(Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body(Body::empty())
                    .unwrap());
            }

            let body = Body::from(bytes);
            Ok(Request::from_parts(parts, body))
        })
    }
}
