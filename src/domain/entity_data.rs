use std::fmt::Debug;

use serde::Serialize;

pub(crate) trait EntityData: Clone + Debug + Serialize + Send + Sync + 'static {}
