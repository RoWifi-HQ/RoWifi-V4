use std::{
    error::Error as StdError,
    fmt::{Display, Formatter},
};
use aws_sdk_dynamodb::Error as DynamoError;

#[derive(Debug)]
pub struct DatabaseError {
    pub(crate) source: Option<Box<dyn StdError + Send + Sync>>,
    pub(crate) kind: ErrorKind,
}

#[derive(Debug)]
pub enum ErrorKind {
    Dynamo,
}

impl DatabaseError {
    pub const fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub fn into_source(self) -> Option<Box<dyn StdError + Send + Sync>> {
        self.source
    }

    pub fn into_parts(self) -> (ErrorKind, Option<Box<dyn StdError + Send + Sync>>) {
        (self.kind, self.source)
    }
}

impl From<DynamoError> for DatabaseError {
    fn from(value: DynamoError) -> Self {
        DatabaseError {
            source: Some(Box::new(value)),
            kind: ErrorKind::Dynamo,
        }
    }
}

impl Display for DatabaseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ErrorKind::Dynamo => write!(f, "dynamo error - {:?}", self.source),
        }
    }
}

impl StdError for DatabaseError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_ref()
            .map(|source| &**source as &(dyn StdError + 'static))
    }
}
