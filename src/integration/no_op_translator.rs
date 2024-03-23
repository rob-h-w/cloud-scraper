use crate::domain::entity::Entity;
use crate::domain::entity_data::EntityData;
use crate::domain::entity_translator::EntityTranslator;

#[derive(Clone, Debug)]
pub(crate) struct NoOpTranslator<Type>
where
    Type: EntityData,
{
    _phantom: std::marker::PhantomData<Type>,
}

impl<Type> NoOpTranslator<Type>
where
    Type: EntityData,
{
    pub(crate) fn new() -> NoOpTranslator<Type>
    where
        Type: EntityData,
    {
        NoOpTranslator {
            _phantom: Default::default(),
        }
    }
}

impl<Type> EntityTranslator<Type, Type> for NoOpTranslator<Type>
where
    Type: EntityData,
{
    fn translate(&self, entity: &Entity<Type>) -> Entity<Type> {
        entity.with_data(entity.data())
    }
}
