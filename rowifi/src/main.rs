#![deny(clippy::all, clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::format_push_string,
    clippy::items_after_statements
)]

mod commands;

use axum::{
    body::Body,
    extract::Request,
    http::{StatusCode, Uri},
    middleware::map_request,
    response::Response,
    routing::post,
    Extension, Json, Router, ServiceExt,
};
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
use std::{error::Error, future::Future, pin::Pin, sync::Arc};
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
    analytics::{analytics_register, analytics_unregister, analytics_view},
    assetbinds::{delete_assetbind, new_assetbind, view_assetbinds},
    audit_log::audit_logs,
    backups::{backup_delete, backup_new, backup_restore, backup_view},
    custombinds::{delete_custombind, new_custombind, view_custombinds},
    denylists::{
        add_custom_denylist, add_group_denylist, add_user_denylist, delete_denylist, view_denylists,
    },
    events::{
        new_event, new_event_type, view_attendee_events, view_event, view_event_types,
        view_host_events,
    },
    groupbinds::{delete_groupbind, new_groupbind, view_groupbinds},
    rankbinds::{delete_rankbind, new_rankbind, view_rankbinds},
    server::{serverinfo, update_all, update_role},
    user::{
        account_default, account_delete, account_switch, account_view, debug_update, update_route,
        userinfo, verify_route,
    },
};

#[tokio::main]
#[allow(clippy::too_many_lines)]
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
    let roblox_proxy = std::env::var("ROBLOX_PROXY").ok();
    let error_logger = std::env::var("ERROR_LOGGER").expect("Expected the error logger");

    let error_logger = twilight_util::link::webhook::parse(&error_logger)?;

    let redis = redis::Client::open(redis_url)?;

    let cache = Cache::new(redis).await?;
    let database = Arc::new(Database::new(&connection_string).await);
    let twilight_http = Arc::new(TwilightClient::new(bot_token.clone()));
    let roblox = RobloxClient::new(&open_cloud_auth, roblox_proxy);
    let bot_context = BotContext::new(
        Id::<ApplicationMarker>::new(application_id),
        twilight_http,
        database,
        cache,
        roblox,
        (error_logger.0, error_logger.1.unwrap().to_string()),
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
        .route("/custombinds/new", post(new_custombind))
        .route("/custombinds/delete", post(delete_custombind))
        .route("/custombinds/view", post(view_custombinds))
        .route("/denylists/user", post(add_user_denylist))
        .route("/denylists/group", post(add_group_denylist))
        .route("/denylists/custom", post(add_custom_denylist))
        .route("/denylists/delete", post(delete_denylist))
        .route("/denylists/view", post(view_denylists))
        .route("/account/view", post(account_view))
        .route("/account/default", post(account_default))
        .route("/account/switch", post(account_switch))
        .route("/account/delete", post(account_delete))
        .route("/verify", post(verify_route))
        .route("/userinfo", post(userinfo))
        .route("/event/new", post(new_event))
        .route("/event/attendee", post(view_attendee_events))
        .route("/event/host", post(view_host_events))
        .route("/event/view", post(view_event))
        .route("/event-types/new", post(new_event_type))
        .route("/event-types/view", post(view_event_types))
        .route("/backup/new", post(backup_new))
        .route("/backup/restore", post(backup_restore))
        .route("/backup/view", post(backup_view))
        .route("/backup/delete", post(backup_delete))
        .route("/update-all", post(update_all))
        .route("/update-role", post(update_role))
        .route("/analytics/view", post(analytics_view))
        .route("/analytics/register", post(analytics_register))
        .route("/analytics/unregister", post(analytics_unregister))
        .route("/serverinfo", post(serverinfo))
        .route("/debug/update", post(debug_update))
        .route("/audit-logs", post(audit_logs))
        .route("/standby", post(standby_route));

    #[cfg(feature = "tower")]
    {
        use rowifi_tower::{
            commands::{
                custom, group_accept, group_decline, set_rank, xp_add, xp_bind_add, xp_bind_delete,
                xp_binds_view, xp_lock, xp_remove, xp_set, xp_unlock, xp_view,
            },
            init_tower,
        };
        router = router
            .route("/setrank", post(set_rank))
            .route("/xp/add", post(xp_add))
            .route("/xp/remove", post(xp_remove))
            .route("/xp/set", post(xp_set))
            .route("/xp/lock", post(xp_lock))
            .route("/xp/unlock", post(xp_unlock))
            .route("/view/xp", post(xp_view))
            .route("/xpbinds/add", post(xp_bind_add))
            .route("/xpbinds/delete", post(xp_bind_delete))
            .route("/xpbinds/view", post(xp_binds_view))
            .route("/group/accept", post(group_accept))
            .route("/group/decline", post(group_decline))
            .route("/{command_name}", post(custom));
        router = init_tower(router);
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
            let Some(InteractionData::MessageComponent(data)) = &interaction.data else {
                unreachable!()
            };
            tracing::trace!("received interacton from component {}", data.custom_id);
            let path = match data.custom_id.as_str() {
                "update" => data.custom_id.as_str(),
                _ => "standby",
            };
            let mut uri_parts = parts.uri.into_parts();
            uri_parts.path_and_query = Some(format!("/{path}").parse().unwrap());
            let new_uri = Uri::from_parts(uri_parts).unwrap();
            parts.uri = new_uri;
        }
        InteractionType::ApplicationCommandAutocomplete => {
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
                format!("/{}/{subcommand_name}/autocomplete", data.name)
            } else {
                format!("/{}/autocomplete", data.name)
            };
            let mut uri_parts = parts.uri.into_parts();
            uri_parts.path_and_query = Some(command_name.parse().unwrap());
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
