use std::{
    error::Error as StdError,
    fmt::{Display, Formatter},
};
use deadpool_postgres::PoolError;
use tokio_postgres::Error as PostgresError;

#[derive(Debug)]
pub struct DatabaseError {
    pub(crate) source: Option<Box<dyn StdError + Send + Sync>>,
    pub(crate) kind: ErrorKind,
}

#[derive(Debug)]
pub enum ErrorKind {
    Pool,
    Postgres,
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

impl From<PoolError> for DatabaseError {
    fn from(value: PoolError) -> Self {
        DatabaseError {
            source: Some(Box::new(value)),
            kind: ErrorKind::Pool,
        }
    }
}

impl From<PostgresError> for DatabaseError {
    fn from(value: PostgresError) -> Self {
        DatabaseError {
            source: Some(Box::new(value)),
            kind: ErrorKind::Postgres,
        }
    }
}

impl Display for DatabaseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ErrorKind::Postgres => write!(f, "postgres error - {:?}", self.source),
            ErrorKind::Pool => write!(f, "pool error - {:?}", self.source),
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
