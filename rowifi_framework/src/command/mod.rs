mod builder;

use rowifi_models::discord::application::interaction::application_command::CommandOptionValue;
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::Service;

use crate::{error::FrameworkError, Request};

use self::builder::CommandBuilder;

type BoxedService = Box<
    dyn Service<
            Request,
            Response = (),
            Error = FrameworkError,
            Future = Pin<Box<dyn Future<Output = Result<(), FrameworkError>> + Send>>,
        > + Send,
>;

pub enum Command {
    Group(CommandGroup),
    Node(CommandNode),
}

pub enum CommandType {
    Group,
    Node,
}

pub struct CommandNode {
    pub name: String,
    pub(crate) service: BoxedService,
}

pub struct CommandGroup {
    pub name: String,
    pub subcommands: HashMap<String, Command>,
}

impl Command {
    #[must_use]
    pub fn kind(&self) -> CommandType {
        match self {
            Command::Group(_) => CommandType::Group,
            Command::Node(_) => CommandType::Node,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Command::Group(group) => &group.name,
            Command::Node(node) => &node.name,
        }
    }

    #[must_use]
    pub fn builder() -> CommandBuilder {
        CommandBuilder {}
    }
}

impl Service<Request> for Command {
    type Response = ();
    type Error = FrameworkError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut req: Request) -> Self::Future {
        match self {
            Command::Group(group) => {
                for option in req.interaction.data {
                    match &option.value {
                        CommandOptionValue::SubCommand(options)
                        | CommandOptionValue::SubCommandGroup(options) => {
                            if let Some(sub_cmd) = group.subcommands.get_mut(option.name.as_str()) {
                                req.interaction.data = options.clone();
                                return sub_cmd.call(req);
                            }
                        }
                        _ => {}
                    }
                }
                Box::pin(async move { Ok(()) })
            }
            Command::Node(node) => {
                let command_fut = node.service.call(req);
                let fut = async move {
                    let res = command_fut.await;
                    if let Err(err) = res {
                        tracing::error!("{}", err);
                    }
                    Ok(())
                };
                Box::pin(fut)
            }
        }
    }
}
