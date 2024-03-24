use hyper::{http::Error as HttpError, Error as HyperError, StatusCode};
use hyper_util::client::legacy::Error as HyperUtilError;
use serde_json::Error as SerdeError;
use std::{
    error::Error as StdError,
    fmt::{Display, Formatter, Result as FmtResult},
};

#[derive(Debug)]
pub enum Error {
    BuildingRequest(HttpError),
    Sending(HyperUtilError),
    Request(HyperError),
    Parsing(SerdeError),
    APIError(StatusCode),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Error::APIError(err) => write!(f, "API Error - {err}"),
            Error::BuildingRequest(err) => write!(f, "Building Request Error - {err}"),
            Error::Parsing(err) => write!(f, "Parsing Error - {err}"),
            Error::Request(err) => write!(f, "Request Error - {err}"),
            Error::Sending(err) => write!(f, "Sending Error - {err}"),
        }
    }
}

impl From<HttpError> for Error {
    fn from(err: HttpError) -> Self {
        Error::BuildingRequest(err)
    }
}

impl From<HyperError> for Error {
    fn from(err: HyperError) -> Self {
        Error::Request(err)
    }
}

impl From<HyperUtilError> for Error {
    fn from(err: HyperUtilError) -> Self {
        Error::Sending(err)
    }
}

impl From<SerdeError> for Error {
    fn from(err: SerdeError) -> Self {
        Error::Parsing(err)
    }
}

impl StdError for Error {}