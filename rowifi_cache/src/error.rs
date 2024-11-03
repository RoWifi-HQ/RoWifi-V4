use redis::RedisError;
use rmp_serde::{decode::Error as SerdeDecodeError, encode::Error as SerdeEncodeError};
use std::{
    error::Error as StdError,
    fmt::{Debug, Display, Formatter, Result as FmtResult},
};

#[derive(Debug)]
pub enum CacheError {
    Redis(RedisError),
    SerdeEncode(SerdeEncodeError),
    SerdeDecode(SerdeDecodeError),
}

impl Display for CacheError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Redis(err) => write!(f, "redis error - {err}"),
            Self::SerdeEncode(err) => write!(f, "serde encoding error - {err}"),
            Self::SerdeDecode(err) => write!(f, "serde decoding error - {err}"),
        }
    }
}

impl StdError for CacheError {}

impl From<SerdeEncodeError> for CacheError {
    fn from(err: SerdeEncodeError) -> Self {
        CacheError::SerdeEncode(err)
    }
}

impl From<SerdeDecodeError> for CacheError {
    fn from(err: SerdeDecodeError) -> Self {
        CacheError::SerdeDecode(err)
    }
}

impl From<RedisError> for CacheError {
    fn from(err: RedisError) -> Self {
        CacheError::Redis(err)
    }
}
