use rowifi_cache::error::CacheError;
use rowifi_database::DatabaseError;
use rowifi_roblox::error::RobloxError;
use std::{
    error::Error as StdError,
    fmt::{Display, Formatter, Result as FmtResult},
};
use twilight_http::{response::DeserializeBodyError, Error as DiscordHttpError};
use twilight_validate::message::MessageValidationError;

#[derive(Debug)]
pub struct RoError {
    source: Option<Box<dyn StdError + Send + Sync>>,
    kind: ErrorKind,
}

#[derive(Debug)]
pub enum ErrorKind {
    Cache,
    Database,
    Discord,
    Function,
    Roblox,
}

impl RoError {
    #[must_use]
    pub const fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    #[must_use]
    pub fn into_source(self) -> Option<Box<dyn StdError + Send + Sync>> {
        self.source
    }

    #[must_use]
    pub fn into_parts(self) -> (ErrorKind, Option<Box<dyn StdError + Send + Sync>>) {
        (self.kind, self.source)
    }

    #[must_use]
    pub fn from_parts(kind: ErrorKind, source: Option<Box<dyn StdError + Send + Sync>>) -> Self {
        Self { source, kind }
    }
}

impl Display for RoError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self.kind {
            ErrorKind::Cache => f.write_str("cache error: ")?,
            ErrorKind::Database => f.write_str("database error: ")?,
            ErrorKind::Discord => f.write_str("discord error: ")?,
            ErrorKind::Function => f.write_str("function error: ")?,
            ErrorKind::Roblox => f.write_str("roblox error: ")?,
        }
        match &self.source {
            Some(err) => Display::fmt(&err, f),
            None => f.write_str(""),
        }
    }
}

impl StdError for RoError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_ref()
            .map(|source| &**source as &(dyn StdError + 'static))
    }
}

impl From<CacheError> for RoError {
    fn from(err: CacheError) -> Self {
        Self {
            source: Some(Box::new(err)),
            kind: ErrorKind::Cache,
        }
    }
}

impl From<DiscordHttpError> for RoError {
    fn from(err: DiscordHttpError) -> Self {
        Self {
            source: Some(Box::new(err)),
            kind: ErrorKind::Discord,
        }
    }
}

impl From<DeserializeBodyError> for RoError {
    fn from(err: DeserializeBodyError) -> Self {
        Self {
            source: Some(Box::new(err)),
            kind: ErrorKind::Discord,
        }
    }
}

impl From<DatabaseError> for RoError {
    fn from(err: DatabaseError) -> Self {
        Self {
            source: Some(Box::new(err)),
            kind: ErrorKind::Database,
        }
    }
}

impl From<RobloxError> for RoError {
    fn from(err: RobloxError) -> Self {
        Self {
            source: Some(Box::new(err)),
            kind: ErrorKind::Roblox,
        }
    }
}

impl From<MessageValidationError> for RoError {
    fn from(err: MessageValidationError) -> Self {
        Self {
            source: Some(Box::new(err)),
            kind: ErrorKind::Function,
        }
    }
}
