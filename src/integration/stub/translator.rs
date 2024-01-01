use crate::domain::config::Config;
use std::rc::Rc;
use uuid::Uuid;

use crate::domain::entity::Entity;
use crate::domain::entity_translator::EntityTranslator;

pub(crate) struct UuidToStringTranslator;

impl EntityTranslator<Uuid, String> for UuidToStringTranslator {
    fn new(_: Rc<impl Config>) -> Box<UuidToStringTranslator> {
        Box::new(UuidToStringTranslator {})
    }

    fn translate(&self, entity: &Entity<Uuid>) -> Entity<String> {
        entity.with_data(&entity.data().to_string())
    }
}
