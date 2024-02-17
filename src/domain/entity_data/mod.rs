use std::fmt::Debug;

use serde::Serialize;

pub(crate) mod string_entity_data;
pub(crate) mod uuid_entity_data;

pub(crate) trait EntityData: Clone + Debug + Serialize + Send + Sync + 'static {}
