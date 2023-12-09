mod arguments;
pub mod command;
pub mod context;
pub mod error;
mod handler;

use futures_util::{
    future::{ready, Either, Ready},
    Future,
};
use rowifi_models::{
    discord::{
        application::interaction::{
            application_command::CommandDataOption, InteractionData, InteractionType,
        },
        gateway::event::Event,
    },
    id::{ChannelId, GuildId},
};
use std::{
    collections::HashMap,
    pin::Pin,
    task::{Context, Poll},
};
use tower::Service;

use crate::{
    command::Command,
    context::{BotContext, CommandContext},
    error::FrameworkError,
};

pub struct Request {
    pub context: CommandContext,
    pub interaction: Interaction,
}

pub struct Interaction {
    pub data: Vec<CommandDataOption>,
}

pub struct Framework {
    bot: BotContext,
    commands: HashMap<String, Command>,
}

impl Framework {
    pub fn new(bot: BotContext) -> Self {
        Self {
            bot,
            commands: HashMap::new()
        }
    }
}

impl Service<&Event> for Framework {
    type Response = ();
    type Error = FrameworkError;
    type Future = Either<
        Ready<Result<Self::Response, Self::Error>>,
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>,
    >;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: &Event) -> Self::Future {
        match req {
            Event::InteractionCreate(interaction) => {
                if interaction.kind == InteractionType::ApplicationCommand {
                    let member = match interaction.member.clone() {
                        Some(m) => m,
                        None => return Either::Left(ready(Ok(()))),
                    };
                    let interaction_data = match &interaction.data {
                        Some(InteractionData::ApplicationCommand(c)) => c,
                        _ => return Either::Left(ready(Ok(()))),
                    };
                    let command = match self.commands.get_mut(interaction_data.name.as_str()) {
                        Some(c) => c,
                        None => return Either::Left(ready(Ok(()))),
                    };

                    let ctx = CommandContext {
                        bot: self.bot.clone(),
                        guild_id: interaction.guild_id.map(GuildId).unwrap(),
                        channel_id: interaction
                            .channel
                            .as_ref()
                            .map(|c| c.id)
                            .map(ChannelId)
                            .unwrap(),
                        member,
                        interaction_id: interaction.id,
                        interaction_token: interaction.token.clone(),
                        resolved: interaction_data.resolved.clone().unwrap(),
                    };
                    let interaction = Interaction {
                        data: interaction_data.options.clone(),
                    };
                    let req = Request {
                        context: ctx,
                        interaction,
                    };

                    return Either::Right(command.call(req));
                }
            }
            _ => {}
        }
        todo!()
    }
}
