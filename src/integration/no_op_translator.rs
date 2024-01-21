use std::sync::Arc;

use crate::domain::config::Config;
use crate::domain::entity::Entity;
use crate::domain::entity_data::EntityData;
use crate::domain::entity_translator::EntityTranslator;
use crate::domain::source::Source;

pub(crate) struct NoOpTranslator;

impl NoOpTranslator {
    pub(crate) fn new<Type>(_: &dyn Source<Type>) -> NoOpTranslator
    where
        Type: EntityData,
    {
        NoOpTranslator {}
    }
}

impl Clone for NoOpTranslator {
    fn clone(&self) -> Self {
        NoOpTranslator
    }
}

impl<Type> EntityTranslator<Type, Type> for NoOpTranslator
where
    Type: EntityData,
{
    fn new(_config: Arc<impl Config>) -> Self {
        NoOpTranslator
    }

    fn translate(&self, entity: &Entity<Type>) -> Entity<Type> {
        entity.with_data(&entity.data())
    }
}
