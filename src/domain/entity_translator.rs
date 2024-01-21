use std::any::TypeId;
use std::fmt::Debug;
use std::sync::Arc;

use crate::domain::config::Config;
use crate::domain::entity::Entity;
use crate::domain::entity_data::EntityData;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct TranslationDescription {
    pub(crate) from: TypeId,
    pub(crate) to: TypeId,
}

pub(crate) trait EntityTranslator<FromDataType, ToDataType>: Clone + Sync
where
    FromDataType: EntityData,
    ToDataType: EntityData,
{
    fn new(config: Arc<impl Config>) -> Self
    where
        Self: Sized;

    fn translation_description() -> TranslationDescription {
        TranslationDescription {
            from: TypeId::of::<FromDataType>(),
            to: TypeId::of::<ToDataType>(),
        }
    }

    fn translate(&self, entity: &Entity<FromDataType>) -> Entity<ToDataType>;
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::integration::stub::source::StubSource;

    use super::*;

    #[derive(Clone)]
    pub(crate) struct TestTranslator;

    impl EntityTranslator<Uuid, String> for TestTranslator {
        fn new(_: Arc<impl Config>) -> Self
        where
            Self: Sized,
        {
            Self
        }

        fn translate(&self, entity: &Entity<Uuid>) -> Entity<String> {
            entity.with_data(&entity.data().to_string())
        }
    }

    #[test]
    fn test_translate() {
        let translator = TestTranslator;
        let uuid = Uuid::new_v4();
        let entity = Entity::new_now::<StubSource>(Box::new(uuid), "1");

        let translated_entity: Entity<String> = translator.translate(&entity);

        assert_eq!(translated_entity.created_at(), entity.created_at());
        assert_eq!(translated_entity.data(), uuid.to_string().as_str());
        assert_eq!(translated_entity.id(), entity.id());
        assert_eq!(translated_entity.updated_at(), entity.updated_at());
    }

    #[test]
    fn test_translation_description() {
        assert_eq!(
            TestTranslator::translation_description(),
            TranslationDescription {
                from: TypeId::of::<Uuid>(),
                to: TypeId::of::<String>(),
            }
        );
    }
}
