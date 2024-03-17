use std::fmt::Debug;

use serde::Serialize;

use crate::domain::entity_data::EntityData;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct GoogleKeep {}

impl EntityData for GoogleKeep {}
