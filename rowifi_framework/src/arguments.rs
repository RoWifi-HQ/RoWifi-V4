use rowifi_models::{
    discord::application::interaction::application_command::{
        CommandDataOption, CommandOptionValue,
    },
    id::UserId,
};
use std::{
    error::Error as StdError,
    fmt::{Display, Formatter, Result as FmtResult},
};

#[allow(dead_code)]
#[derive(Debug)]
pub enum ArgumentError {
    BadArgument,
}

pub trait Arguments {
    /// Converts the interaction data into a struct implementing Arguments
    ///
    /// # Errors
    ///
    /// Return Err if any of the arguments cannot be parsed.
    fn from_interaction(options: &[CommandDataOption]) -> Result<Self, ArgumentError>
    where
        Self: Sized;
}

pub trait Argument {
    /// Converts a single field of the interaction data into an Argument struct
    ///
    /// # Errors
    ///
    /// Return Err if the data cannot be parsed.
    fn from_interaction(option: &CommandDataOption) -> Result<Self, ArgumentError>
    where
        Self: Sized;
}

impl Arguments for () {
    fn from_interaction(_: &[CommandDataOption]) -> Result<Self, ArgumentError> {
        Ok(())
    }
}

impl<T: Arguments> Arguments for (T,) {
    fn from_interaction(options: &[CommandDataOption]) -> Result<Self, ArgumentError> {
        match T::from_interaction(options) {
            Ok(a) => Ok((a,)),
            Err(err) => Err(err),
        }
    }
}

impl<T: Argument> Argument for Option<T> {
    fn from_interaction(option: &CommandDataOption) -> Result<Self, ArgumentError> {
        Ok(T::from_interaction(option).ok())
    }
}

impl Argument for UserId {
    fn from_interaction(option: &CommandDataOption) -> Result<Self, ArgumentError> {
        match &option.value {
            #[allow(clippy::cast_sign_loss)]
            CommandOptionValue::Integer(value) => Ok(UserId::new(*value as u64)),
            CommandOptionValue::User(value) => Ok(UserId(*value)),
            _ => unreachable!("UserId unreached"),
        }
    }
}

impl Display for ArgumentError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            ArgumentError::BadArgument => write!(f, "argument error"),
        }
    }
}

impl StdError for ArgumentError {}
