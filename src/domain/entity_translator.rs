use std::any::TypeId;

use crate::domain::config::Config;
use crate::domain::entity::Entity;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct TranslationDescription {
    pub(crate) from: TypeId,
    pub(crate) to: TypeId,
}

pub(crate) trait EntityTranslator<T: 'static, U: 'static> {
    fn new(config: impl Config) -> Self;

    fn translation_description(&self) -> TranslationDescription {
        TranslationDescription {
            from: TypeId::of::<T>(),
            to: TypeId::of::<U>(),
        }
    }

    fn translate(&self, entity: &Entity<T>) -> Entity<U>;
}

#[cfg(test)]
pub(crate) mod tests {
    use uuid::Uuid;

    use crate::integration::stub::source::StubSource;

    use super::*;

    pub(crate) struct TestTranslator;

    impl EntityTranslator<Uuid, String> for TestTranslator {
        fn new(_: impl Config) -> Self {
            Self
        }

        fn translate(&self, entity: &Entity<Uuid>) -> Entity<String> {
            entity.with_data(&entity.data().to_string())
        }
    }

    #[test]
    fn test_translate() {
        let translator = TestTranslator;
        let source = StubSource::new();
        let uuid = Uuid::new_v4();
        let entity = Entity::new_now(Box::new(uuid), "1", &source);

        let translated_entity: Entity<String> = translator.translate(&entity);

        assert_eq!(translated_entity.created_at(), entity.created_at());
        assert_eq!(translated_entity.data(), uuid.to_string().as_str());
        assert_eq!(translated_entity.id(), entity.id());
        assert_eq!(translated_entity.updated_at(), entity.updated_at());
    }

    #[test]
    fn test_translation_description() {
        let translator = TestTranslator;
        assert_eq!(
            translator.translation_description(),
            TranslationDescription {
                from: TypeId::of::<Uuid>(),
                to: TypeId::of::<String>(),
            }
        );
    }
}
