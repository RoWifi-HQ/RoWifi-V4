#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::similar_names, clippy::module_name_repetitions)]

pub mod arguments;
pub mod context;
pub mod prelude;

use std::sync::atomic::AtomicBool;

use axum::{
    async_trait,
    body::Body,
    extract::FromRequest,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};
use rowifi_models::{
    discord::application::interaction::{
        application_command::{CommandDataOption, CommandOptionValue},
        Interaction, InteractionData, InteractionType,
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
        let (_parts, body) = req.into_parts();
        let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();

        let interaction = serde_json::from_slice::<Interaction>(&bytes)
            .map_err(|_err| StatusCode::BAD_REQUEST.into_response())?;
        if interaction.kind == InteractionType::ApplicationCommand {
            let Some(InteractionData::ApplicationCommand(data)) = &interaction.data else {
                unreachable!()
            };
            let ctx = CommandContext {
                guild_id: GuildId(interaction.guild_id.unwrap()),
                channel_id: ChannelId(interaction.channel.as_ref().unwrap().id),
                author_id: UserId(interaction.author_id().unwrap()),
                interaction_id: interaction.id,
                interaction_token: interaction.token,
                resolved: data.resolved.clone(),
                callback_invoked: AtomicBool::new(false),
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
