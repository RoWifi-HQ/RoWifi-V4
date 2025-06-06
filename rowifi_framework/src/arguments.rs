use rowifi_models::{
    audit_log::AuditLogKind,
    bind::AssetType,
    deny_list::DenyListActionType,
    discord::{
        application::interaction::application_command::{CommandDataOption, CommandOptionValue},
        id::{marker::RoleMarker, Id},
    },
    id::{RoleId, UserId},
    roblox::id::GroupId,
};
use std::{
    error::Error as StdError,
    fmt::{Display, Formatter, Result as FmtResult},
};
use twilight_mention::ParseMention;

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

impl Argument for CommandDataOption {
    fn from_interaction(option: &CommandDataOption) -> Result<Self, ArgumentError> {
        Ok(option.clone())
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

impl Argument for u8 {
    fn from_interaction(option: &CommandDataOption) -> Result<Self, ArgumentError> {
        match &option.value {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            CommandOptionValue::Integer(value) => Ok(*value as Self),
            _ => unreachable!("u8 reached"),
        }
    }
}

impl Argument for u32 {
    fn from_interaction(option: &CommandDataOption) -> Result<Self, ArgumentError> {
        match &option.value {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            CommandOptionValue::Integer(value) => Ok(*value as Self),
            _ => unreachable!("u32 reached"),
        }
    }
}

impl Argument for u64 {
    fn from_interaction(option: &CommandDataOption) -> Result<Self, ArgumentError> {
        match &option.value {
            #[allow(clippy::cast_sign_loss)]
            CommandOptionValue::Integer(value) => Ok(*value as Self),
            CommandOptionValue::Role(value) => Ok(value.get()),
            _ => unreachable!("u64 reached"),
        }
    }
}

impl Argument for i32 {
    fn from_interaction(option: &CommandDataOption) -> Result<Self, ArgumentError> {
        match &option.value {
            #[allow(clippy::cast_possible_truncation)]
            CommandOptionValue::Integer(value) => Ok(*value as Self),
            _ => unreachable!("i32 reached"),
        }
    }
}

impl Argument for String {
    fn from_interaction(option: &CommandDataOption) -> Result<Self, ArgumentError> {
        match &option.value {
            CommandOptionValue::String(value) => Ok(value.clone()),
            _ => unreachable!("String reached"),
        }
    }
}

impl Argument for Vec<RoleId> {
    fn from_interaction(option: &CommandDataOption) -> Result<Self, ArgumentError> {
        let roles = match &option.value {
            CommandOptionValue::String(value) => Id::<RoleMarker>::iter(value)
                .map(|v| RoleId(v.0))
                .collect::<Vec<_>>(),
            _ => unreachable!("Vec reached"),
        };

        Ok(roles)
    }
}

impl Argument for AssetType {
    fn from_interaction(option: &CommandDataOption) -> Result<Self, ArgumentError> {
        match &option.value {
            CommandOptionValue::Integer(value) => match value {
                0 => Ok(AssetType::Asset),
                1 => Ok(AssetType::Badge),
                2 => Ok(AssetType::Gamepass),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }
}

impl Argument for DenyListActionType {
    fn from_interaction(option: &CommandDataOption) -> Result<Self, ArgumentError> {
        match &option.value {
            CommandOptionValue::Integer(value) => match value {
                0 => Ok(DenyListActionType::None),
                1 => Ok(DenyListActionType::Kick),
                2 => Ok(DenyListActionType::Ban),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }
}

impl Argument for RoleId {
    fn from_interaction(option: &CommandDataOption) -> Result<Self, ArgumentError> {
        match option.value {
            CommandOptionValue::Role(role) => Ok(RoleId(role)),
            _ => unreachable!(),
        }
    }
}

impl Argument for GroupId {
    fn from_interaction(option: &CommandDataOption) -> Result<Self, ArgumentError> {
        match option.value {
            #[allow(clippy::cast_sign_loss)]
            CommandOptionValue::Integer(group) => Ok(GroupId(group as u64)),
            _ => unreachable!(),
        }
    }
}

impl Argument for bool {
    fn from_interaction(option: &CommandDataOption) -> Result<Self, ArgumentError> {
        match option.value {
            CommandOptionValue::Boolean(bool) => Ok(bool),
            _ => unreachable!(),
        }
    }
}

impl Argument for AuditLogKind {
    fn from_interaction(option: &CommandDataOption) -> Result<Self, ArgumentError> {
        match option.value {
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            CommandOptionValue::Integer(kind) => Ok(AuditLogKind::try_from(kind as u32).unwrap()),
            _ => unreachable!(),
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
