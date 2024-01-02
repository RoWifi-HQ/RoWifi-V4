use rowifi_cache::error::CacheError;
use rowifi_database::DatabaseError;
use rowifi_roblox::error::RobloxError;
use std::{
    error::Error as StdError,
    fmt::{Display, Formatter, Result as FmtResult},
};
use twilight_http::{response::DeserializeBodyError, Error as DiscordHttpError};

use crate::arguments::ArgumentError;

#[derive(Debug)]
pub struct FrameworkError {
    source: Option<Box<dyn StdError + Send + Sync>>,
    kind: ErrorKind,
}

#[derive(Debug)]
pub enum ErrorKind {
    Cache,
    Command,
    Database,
    Discord,
    Roblox,
}

impl FrameworkError {
    pub const fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub fn into_source(self) -> Option<Box<dyn StdError + Send + Sync>> {
        self.source
    }

    pub fn into_parts(self) -> (ErrorKind, Option<Box<dyn StdError + Send + Sync>>) {
        (self.kind, self.source)
    }

    pub fn from_parts(kind: ErrorKind, source: Option<Box<dyn StdError + Send + Sync>>) -> Self {
        Self { kind, source }
    }
}

impl Display for FrameworkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self.kind {
            ErrorKind::Cache => f.write_str("cache error: ")?,
            ErrorKind::Command => f.write_str("command error: ")?,
            ErrorKind::Database => f.write_str("database error: ")?,
            ErrorKind::Discord => f.write_str("discord error: ")?,
            ErrorKind::Roblox => f.write_str("roblox error: ")?,
        }
        match &self.source {
            Some(err) => Display::fmt(&err, f),
            None => f.write_str(""),
        }
    }
}

impl StdError for FrameworkError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_ref()
            .map(|source| &**source as &(dyn StdError + 'static))
    }
}

impl From<ArgumentError> for FrameworkError {
    fn from(err: ArgumentError) -> Self {
        Self {
            source: Some(Box::new(err)),
            kind: ErrorKind::Command,
        }
    }
}

impl From<CacheError> for FrameworkError {
    fn from(err: CacheError) -> Self {
        Self {
            source: Some(Box::new(err)),
            kind: ErrorKind::Cache,
        }
    }
}

impl From<DiscordHttpError> for FrameworkError {
    fn from(err: DiscordHttpError) -> Self {
        Self {
            source: Some(Box::new(err)),
            kind: ErrorKind::Discord,
        }
    }
}

impl From<DeserializeBodyError> for FrameworkError {
    fn from(err: DeserializeBodyError) -> Self {
        Self {
            source: Some(Box::new(err)),
            kind: ErrorKind::Discord,
        }
    }
}

impl From<DatabaseError> for FrameworkError {
    fn from(err: DatabaseError) -> Self {
        Self {
            source: Some(Box::new(err)),
            kind: ErrorKind::Database,
        }
    }
}

impl From<RobloxError> for FrameworkError {
    fn from(err: RobloxError) -> Self {
        Self {
            source: Some(Box::new(err)),
            kind: ErrorKind::Roblox,
        }
    }
}
