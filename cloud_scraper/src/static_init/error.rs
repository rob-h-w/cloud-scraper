use crate::static_init::error::Error::{Io, TokenRequestFailed};
use core::error::Error as CoreError;
use log::{debug, error};
use oauth2::basic::BasicErrorResponse;
use oauth2::RequestTokenError;
use serde_yaml::Value;
use std::error;
use std::error::Error as StdError;
use std::fmt::{Display, Formatter};
use std::io;
use tokio::task::JoinError;
use warp::reject::Reject;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Cancelled(String),
    Connection(String),
    FailedAfterRetries,
    Io(String),
    KeyNotFound(Value),
    NotAMapping(Value),
    Oauth2CodeMissing,
    Oauth2CsrfMismatch,
    Oauth2TokenAbsent,
    Oauth2TokenExpired,
    Panicked(String),
    TokenRequestFailed,
    YamlSerialization(String),
}

impl Reject for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Cancelled(e) => write!(f, "Cancelled: {}", e),
            Error::Connection(e) => write!(f, "Connection error: {}", e),
            Error::FailedAfterRetries => write!(f, "Failed after retries"),
            Io(e) => write!(f, "IO error: {}", e),
            Error::KeyNotFound(v) => write!(f, "Key not found: {:?}", v),
            Error::NotAMapping(v) => write!(f, "Not a mapping: {:?}", v),
            Error::Oauth2CodeMissing => write!(f, "Oauth2 code missing"),
            Error::Oauth2CsrfMismatch => write!(f, "Oauth2 CSRF mismatch"),
            Error::Oauth2TokenAbsent => write!(f, "Oauth2 token absent"),
            Error::Oauth2TokenExpired => write!(f, "Oauth2 token expired"),
            Error::Panicked(e) => write!(f, "Panicked: {}", e),
            TokenRequestFailed => write!(f, "Token request failed"),
            Error::YamlSerialization(e) => write!(f, "YAML serialization error: {}", e),
        }
    }
}

impl error::Error for Error {}

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
    fn to_error(&self) -> Error;
}

impl IoErrorExt for io::Error {
    fn to_error(&self) -> Error {
        Io(self.to_string())
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_error_is_send_and_sync() {
        fn is_send_and_sync<T: Send + Sync>(_candidate: &T) {}

        is_send_and_sync(&super::Error::Io("test".to_string()));
    }
}
