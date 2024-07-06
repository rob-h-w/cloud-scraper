#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
use crate::domain::config::Config;

#[cfg(test)]
use crate::domain::entity::Entity;

#[cfg(test)]
use crate::domain::entity_data::EntityData;

#[cfg(test)]
use crate::domain::entity_translator::EntityTranslator;

#[cfg(test)]
use crate::domain::source::Source;

#[cfg(test)]
pub(crate) struct NoOpTranslator;

#[cfg(test)]
impl NoOpTranslator {
    pub(crate) fn new<SourceType, Type>(_: &Arc<SourceType>) -> NoOpTranslator
    where
        SourceType: Source<Type>,
        Type: EntityData,
    {
        NoOpTranslator {}
    }
}

#[cfg(test)]
impl Clone for NoOpTranslator {
    fn clone(&self) -> Self {
        NoOpTranslator
    }
}

#[cfg(test)]
impl<Type> EntityTranslator<Type, Type> for NoOpTranslator
where
    Type: EntityData,
{
    #[cfg(test)]
    fn new(_config: Arc<impl Config>) -> Self {
        NoOpTranslator
    }

    fn translate(&self, entity: &Entity<Type>) -> Entity<Type> {
        entity.with_data(entity.data())
    }
}
