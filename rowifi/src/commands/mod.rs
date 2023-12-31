use rowifi_framework::error::{ErrorKind, FrameworkError};
use rowifi_models::discord::channel::message::Embed;
use std::{
    error::Error as StdError,
    fmt::{Display, Formatter, Result as FmtResult},
};

pub mod user;

#[derive(Debug)]
pub struct CommandError {
    pub kind: CommandErrorType,
    pub response: Option<CommandErrorResponse>,
}

#[derive(Debug)]
pub enum CommandErrorResponse {
    Text(String),
    Embed(Box<Embed>),
}

#[derive(Debug)]
pub enum CommandErrorType {
    UserNotFound = 1000,
}

impl Display for CommandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "command error: {:?}", self.kind)
    }
}

impl StdError for CommandError {}

impl From<CommandError> for FrameworkError {
    fn from(err: CommandError) -> Self {
        FrameworkError::from_parts(ErrorKind::Command, Some(Box::new(err)))
    }
}

impl From<(CommandErrorType, String)> for CommandError {
    fn from(value: (CommandErrorType, String)) -> Self {
        Self {
            kind: value.0,
            response: Some(CommandErrorResponse::Text(value.1)),
        }
    }
}
