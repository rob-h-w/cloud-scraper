use serde::Serialize;
use std::fmt::Debug;

pub(crate) trait EntityData: Clone + Debug + Serialize + 'static {}
