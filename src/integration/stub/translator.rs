use uuid::Uuid;

use crate::domain::entity::Entity;
use crate::domain::entity_translator::EntityTranslator;

#[derive(Clone, Debug)]
pub(crate) struct UuidToStringTranslator;

impl EntityTranslator<Uuid, String> for UuidToStringTranslator {
    fn translate(&self, entity: &Entity<Uuid>) -> Entity<String> {
        entity.with_data(&entity.data().to_string())
    }
}
