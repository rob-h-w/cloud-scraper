use warp::Rejection;

pub(crate) type ResultRejection<T> = Result<T, Rejection>;
