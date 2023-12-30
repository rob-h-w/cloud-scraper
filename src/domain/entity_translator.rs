use crate::domain::config::Config;
use std::any::{Any, TypeId};

use crate::domain::entity::Entity;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct TranslationDescription {
    pub(crate) from: TypeId,
    pub(crate) to: TypeId,
}

pub(crate) trait EntityTranslator {
    fn new(config: impl Config) -> Self
    where
        Self: Sized;
    fn translation_description(&self) -> TranslationDescription;

    fn do_translate(&self, entity: &dyn Any) -> Box<dyn Any>;
}

fn translate<T, U>(translator: &impl EntityTranslator, entity: &Entity<T>) -> Entity<U>
where
    T: Clone + 'static,
    U: Clone + 'static,
{
    entity.with_data(
        translator
            .do_translate(entity.data())
            .downcast_ref::<U>()
            .unwrap(),
    )
}

#[cfg(test)]
mod tests {
    use crate::integration::stub::source::StubSource;
    use uuid::Uuid;

    use super::*;

    struct TestTranslator;

    impl EntityTranslator for TestTranslator {
        fn new(config: impl Config) -> Self
        where
            Self: Sized,
        {
            TestTranslator
        }

        fn translation_description(&self) -> TranslationDescription {
            TranslationDescription {
                from: TypeId::of::<Uuid>(),
                to: TypeId::of::<String>(),
            }
        }

        fn do_translate(&self, entity: &dyn Any) -> Box<dyn Any> {
            let from = entity.downcast_ref::<Uuid>().unwrap();
            Box::new(from.to_string())
        }
    }

    #[test]
    fn test_translate() {
        let translator = TestTranslator;
        let source = StubSource::new();
        let uuid = Uuid::new_v4();
        let entity = Entity::new_now(Box::new(uuid), "1", &source);

        let translated_entity: Entity<String> = translate(&translator, &entity);

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
