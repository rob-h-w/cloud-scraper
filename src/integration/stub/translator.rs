#[cfg(test)]
use std::sync::Arc;

use uuid::Uuid;

#[cfg(test)]
use crate::domain::config::Config;
use crate::domain::entity::Entity;
use crate::domain::entity_translator::EntityTranslator;

#[derive(Clone)]
pub(crate) struct UuidToStringTranslator;

impl EntityTranslator<Uuid, String> for UuidToStringTranslator {
    #[cfg(test)]
    fn new(_: Arc<impl Config>) -> UuidToStringTranslator {
        UuidToStringTranslator {}
    }

    fn translate(&self, entity: &Entity<Uuid>) -> Entity<String> {
        entity.with_data(&entity.data().to_string())
    }
}
