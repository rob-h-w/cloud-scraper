use crate::static_init::error::Error;
use tokio::sync::broadcast::error::SendError;
use warp::reject::Reject;

#[derive(Debug)]
pub enum Rejection {
    SendRejection(String),
}

impl Reject for Rejection {}

impl<T> From<SendError<T>> for Rejection
where
    T: std::fmt::Debug,
{
    fn from(error: SendError<T>) -> Self {
        Self::SendRejection(format!("{:?}", error))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejection_is_reject() {
        let send_error = SendError(123);
        let expected_message = "SendError(123)";
        match Rejection::from(send_error) {
            Rejection::SendRejection(message) => assert_eq!(message, expected_message.to_string()),
        }
    }
}
