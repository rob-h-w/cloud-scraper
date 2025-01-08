use crate::static_init::error::Error;
use std::{fmt, io};
use tokio::sync::broadcast::error::SendError;
use warp::reject::Reject;

#[derive(Debug)]
pub enum Rejection {
    IoRejection(String),
    SendRejection(String),
}

impl Reject for Rejection {}

impl fmt::Display for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rejection::SendRejection(e) => write!(f, "Send error: {}", e),
            Rejection::IoRejection(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl<T> From<SendError<T>> for Rejection
where
    T: std::fmt::Debug,
{
    fn from(error: SendError<T>) -> Self {
        Self::SendRejection(format!("{:?}", error))
    }
}

impl From<io::Error> for Rejection {
    fn from(error: io::Error) -> Self {
        Self::IoRejection(format!("{:?}", error))
    }
}

pub trait Rejectable {
    fn into_rejection(self) -> warp::Rejection;
}

impl<T> Rejectable for SendError<T>
where
    T: std::fmt::Debug,
{
    fn into_rejection(self) -> warp::Rejection {
        warp::reject::custom(Rejection::from(self))
    }
}

impl Rejectable for Error {
    fn into_rejection(self) -> warp::Rejection {
        warp::reject::custom(self)
    }
}

impl Rejectable for io::Error {
    fn into_rejection(self) -> warp::Rejection {
        warp::reject::custom(Rejection::from(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    #[test]
    fn rejection_is_reject() {
        fn check_rejection(rejection: Rejection, expected_message: &str) {
            match rejection {
                Rejection::SendRejection(message) => {
                    assert_eq!(message, expected_message.to_string());
                }
                Rejection::IoRejection(message) => {
                    assert_eq!(message, expected_message.to_string());
                }
            }
        }

        check_rejection(
            Rejection::from(io::Error::new(ErrorKind::AddrInUse, "test")),
            "Custom { kind: AddrInUse, error: \"test\" }",
        );
        check_rejection(Rejection::from(SendError(123)), "SendError(123)");
    }

    mod display {
        use super::*;

        #[test]
        fn rejection_display() {
            let rejection = Rejection::SendRejection("error".to_string());
            let expected_message = "Send error: error";
            assert_eq!(format!("{}", rejection), expected_message);
        }
    }
}
