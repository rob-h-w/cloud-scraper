use crate::domain::entity_identifier::EntityIdentifier;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Hash, PartialEq, PartialOrd, Serialize)]
pub(crate) struct Entity<T> {
    pub(crate) id: EntityIdentifier,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) data: T,
}
