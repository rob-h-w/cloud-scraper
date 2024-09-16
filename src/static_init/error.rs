use crate::static_init::error::Error::TokenRequestFailed;
use core::error::Error as CoreError;
use log::{debug, error};
use oauth2::basic::BasicErrorResponse;
use oauth2::RequestTokenError;
use serde_yaml::Value;
use std::error::Error as StdError;
use std::io;
use tokio::task::JoinError;
use warp::reject::Reject;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Builder(String),
    Cancelled(String),
    Connection(String),
    FailedAfterRetries,
    KeyNotFound(Value),
    NotAMapping(Value),
    Oauth2CodeMissing,
    Oauth2CsrfMismatch,
    Panicked(String),
    TokenRequestFailed,
    YamlSerialization(String),
}

impl Reject for Error {}

pub(crate) trait RequestTokenErrorExt {
    fn to_error(&self) -> Error;
}

impl<E> RequestTokenErrorExt for RequestTokenError<E, BasicErrorResponse>
where
    E: StdError,
{
    fn to_error(&self) -> Error {
        match self {
            RequestTokenError::ServerResponse(response) => {
                error!("RequestTokenError::ServerResponse {:?}", response);
            }
            RequestTokenError::Request(e) => {
                error!("RequestTokenError::Request {:?}", e);
            }
            RequestTokenError::Parse(e, response) => {
                error!(
                    "RequestTokenError::Parse {:?}, could not parse {:?}",
                    e, response
                );
            }
            RequestTokenError::Other(e) => {
                error!("RequestTokenError::Other {:?}", e);
            }
        }

        TokenRequestFailed
    }
}

pub(crate) trait JoinErrorExt {
    fn to_error(&self) -> Error;
}

impl JoinErrorExt for JoinError {
    fn to_error(&self) -> Error {
        if self.is_cancelled() {
            return Error::Cancelled(self.to_string());
        }
        if self.is_panic() {
            return Error::Panicked(self.to_string());
        }
        match self.source() {
            Some(source) => Error::Connection(source.to_string()),
            None => {
                debug!("JoinError {:?} has no source", self);
                Error::Connection(self.to_string())
            }
        }
    }
}

pub trait SerdeErrorExt {
    fn to_yaml_serialization_error(&self) -> Error;
}

impl SerdeErrorExt for serde_yaml::Error {
    fn to_yaml_serialization_error(&self) -> Error {
        Error::YamlSerialization(self.to_string())
    }
}

pub trait IoErrorExt {
    fn to_source_creation_builder_error(&self) -> Error;
}

impl IoErrorExt for io::Error {
    fn to_source_creation_builder_error(&self) -> Error {
        Error::Builder(self.to_string())
    }
}
