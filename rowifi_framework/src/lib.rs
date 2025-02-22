#![deny(clippy::all, clippy::pedantic)]
#![allow(
    clippy::similar_names,
    clippy::module_name_repetitions,
    clippy::missing_panics_doc
)]

pub mod arguments;
pub mod context;
pub mod prelude;
pub mod utils;

use std::sync::{atomic::AtomicBool, Arc};

use axum::{
    async_trait,
    body::Body,
    extract::FromRequest,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use context::BotContext;
use rowifi_core::error::RoError;
use rowifi_models::{
    discord::{
        application::interaction::{
            application_command::{CommandDataOption, CommandOptionValue},
            Interaction, InteractionData, InteractionType,
        },
        http::interaction::{
            InteractionResponse, InteractionResponseData, InteractionResponseType,
        },
    },
    id::{ChannelId, GuildId, UserId},
};

use crate::{arguments::Arguments, context::CommandContext};

pub struct Command<A> {
    pub ctx: CommandContext,
    pub args: A,
}

#[async_trait]
impl<S, A> FromRequest<S, Body> for Command<A>
where
    S: Send + Sync,
    A: Arguments,
{
    type Rejection = Response;

    async fn from_request(req: Request<Body>, _state: &S) -> Result<Self, Self::Rejection> {
        let (parts, body) = req.into_parts();
        let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();

        let interaction = serde_json::from_slice::<Interaction>(&bytes)
            .map_err(|_err| StatusCode::BAD_REQUEST.into_response())?;
        if interaction.kind == InteractionType::ApplicationCommand {
            let Some(InteractionData::ApplicationCommand(data)) = &interaction.data else {
                unreachable!()
            };
            let Some(guild_id) = interaction.guild_id else {
                return Err(Json(InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        content: Some(
                            "Commands are only allowed to be run in a guild context".into(),
                        ),
                        ..Default::default()
                    }),
                })
                .into_response());
            };
            let ctx = CommandContext {
                name: parts
                    .uri
                    .path_and_query()
                    .map(std::string::ToString::to_string)
                    .unwrap_or_default(),
                guild_id: GuildId(guild_id),
                channel_id: ChannelId(interaction.channel.as_ref().unwrap().id),
                author_id: UserId(interaction.author_id().unwrap()),
                interaction_id: interaction.id,
                interaction_token: interaction.token,
                resolved: data.resolved.clone(),
                callback_invoked: Arc::new(AtomicBool::new(false)),
            };
            let data = recurse_skip_subcommands(&data.options);
            let args = A::from_interaction(data).unwrap();
            Ok(Command { ctx, args })
        } else {
            todo!()
        }
    }
}

fn recurse_skip_subcommands(data: &[CommandDataOption]) -> &[CommandDataOption] {
    if let Some(option) = data.first() {
        match &option.value {
            CommandOptionValue::SubCommand(options)
            | CommandOptionValue::SubCommandGroup(options) => {
                return options;
            }
            _ => return data,
        }
    }

    data
}

pub async fn handle_error(bot: BotContext, ctx: CommandContext, err: RoError) {
    tracing::error!(name =? ctx.name, guild_id = ?ctx.guild_id, err = ?err);
    let _ = ctx.respond(&bot)
        .content("Something went wrong. Please try again. If the issue persists, please contact the RoWifi support server.")
        .unwrap()
        .await;
    let _ = bot
        .http
        .execute_webhook(bot.error_logger.0, &bot.error_logger.1)
        .content(&format!(
            "```
Guild Id: {}
Command: {}
Error: {}
        ```",
            ctx.guild_id, ctx.name, err
        ))
        .await;
}
