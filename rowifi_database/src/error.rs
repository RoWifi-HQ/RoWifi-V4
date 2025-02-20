use deadpool_postgres::PoolError;
use std::{
    error::Error as StdError,
    fmt::{Display, Formatter},
};
use tokio_postgres::Error as PostgresError;

#[derive(Debug)]
pub struct DatabaseError(pub(crate) Box<dyn StdError + Send + Sync>);

impl DatabaseError {
    #[must_use]
    pub fn into_source(self) -> Box<dyn StdError + Send + Sync> {
        self.0
    }
}

impl From<PostgresError> for DatabaseError {
    fn from(err: PostgresError) -> Self {
        Self(Box::new(err))
    }
}

impl From<PoolError> for DatabaseError {
    fn from(err: PoolError) -> Self {
        Self(Box::new(err))
    }
}

impl Display for DatabaseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "database error: {}", self.0)
    }
}

impl StdError for DatabaseError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(self.0.as_ref())
    }
}
