use crate::domain::entity_identifier::EntityIdentifier;
use chrono::{DateTime, Utc};

#[derive(PartialEq, PartialOrd, Debug, Hash)]
pub(crate) struct Entity<T> {
    pub(crate) id: EntityIdentifier,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) data: T,
}
