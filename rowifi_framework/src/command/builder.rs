use std::{collections::HashMap, future::Future};

use crate::{
    arguments::Arguments,
    error::FrameworkError,
    handler::{CommandHandler, Handler},
};

use super::{BoxedService, Command, CommandGroup, CommandNode};

pub struct CommandBuilder;

pub struct CommandGroupBuilder {
    pub name: String,
    pub subcommands: HashMap<String, Command>,
}

pub struct CommandNodeBuilder {
    pub name: String,
}

impl CommandBuilder {
    pub fn group() -> CommandGroupBuilder {
        CommandGroupBuilder {
            name: "".into(),
            subcommands: HashMap::new(),
        }
    }

    pub fn node() -> CommandNodeBuilder {
        CommandNodeBuilder { name: "".into() }
    }
}

impl CommandGroupBuilder {
    /// Set the name of the command
    pub fn name(mut self, name: &str) -> CommandGroupBuilder {
        self.name = name.into();
        self
    }

    /// Add a subcommand to the command group
    pub fn subcommand(mut self, subcommand: Command) -> CommandGroupBuilder {
        self.subcommands
            .insert(subcommand.name().to_string(), subcommand);
        self
    }

    /// Consume the builder
    pub fn build(self) -> Command {
        assert_ne!(self.name, "");

        Command::Group(CommandGroup {
            name: self.name,
            subcommands: self.subcommands,
        })
    }
}

impl CommandNodeBuilder {
    /// Set the name of the command
    pub fn name(mut self, name: &str) -> CommandNodeBuilder {
        self.name = name.into();
        self
    }

    /// Set the function of the command in the form of a [`BoxedService`] and consume the builder
    pub fn service(self, service: BoxedService) -> Command {
        Command::Node(CommandNode {
            name: self.name,
            service,
        })
    }

    /// Set the function of the command and consume the builder
    pub fn handler<F, T, R>(self, handler: F) -> Command
    where
        F: Handler<T, R> + Send + 'static,
        R: Future<Output = Result<(), FrameworkError>> + Send + 'static,
        T: Arguments + Send + 'static,
    {
        Command::Node(CommandNode {
            name: self.name,
            service: Box::new(CommandHandler::new(handler)),
        })
    }
}
