use hyper::StatusCode;
use std::{
    error::Error as StdError,
    fmt::{Display, Formatter, Result as FmtResult},
};

#[derive(Debug)]
pub enum ErrorKind {
    BuildingRequest,
    Sending,
    ChunkingResponse,
    Response {
        route: String,
        status: StatusCode,
        bytes: Vec<u8>,
    },
    Deserialize,
}

#[derive(Debug)]
pub struct RobloxError {
    pub(super) source: Option<Box<dyn StdError + Send + Sync>>,
    pub(super) kind: ErrorKind,
}

#[derive(Debug)]
pub struct DeserializeBodyError {
    pub(super) source: Option<Box<dyn StdError + Send + Sync>>,
    pub(super) bytes: Vec<u8>,
}

impl RobloxError {
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

impl Display for RobloxError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self.kind() {
            ErrorKind::BuildingRequest => write!(f, "failed to build the request"),
            ErrorKind::Sending => write!(f, "sending the request failed"),
            ErrorKind::ChunkingResponse => write!(f, "chunking the response failed"),
            ErrorKind::Response {
                route,
                status,
                bytes: _,
            } => write!(f, "failed with {status} on {route}"),
            ErrorKind::Deserialize => write!(f, "error deserializing"),
        }
    }
}

impl StdError for RobloxError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_ref()
            .map(|source| &**source as &(dyn StdError + 'static))
    }
}

impl Display for DeserializeBodyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let display = String::from_utf8_lossy(&self.bytes);
        write!(f, "bytes: {display}")
    }
}

impl StdError for DeserializeBodyError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_ref()
            .map(|source| &**source as &(dyn StdError + 'static))
    }
}
