use std::any::TypeId;

use crate::domain::entity::Entity;

pub(crate) trait EntityTranslator<T: 'static, U: 'static> {
    fn input_entity(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn output_entity(&self) -> TypeId {
        TypeId::of::<U>()
    }

    fn translate(&self, entity: &Entity<T>) -> Entity<U>;
}

#[cfg(test)]
mod tests {
    use crate::integration::stub::source::StubSource;
    use uuid::Uuid;

    use super::*;

    struct TestTranslator;

    impl EntityTranslator<Uuid, String> for TestTranslator {
        fn translate(&self, entity: &Entity<Uuid>) -> Entity<String> {
            entity.with_data(entity.data().to_string())
        }
    }

    #[test]
    fn test_translate() {
        let translator = TestTranslator;
        let source = StubSource::new();
        let uuid = Uuid::new_v4();
        let entity = Entity::new_now(Box::new(uuid), "1", &source);

        let translated_entity = translator.translate(&entity);

        assert_eq!(translated_entity.created_at(), entity.created_at());
        assert_eq!(translated_entity.data(), uuid.to_string().as_str());
        assert_eq!(translated_entity.id(), entity.id());
        assert_eq!(translated_entity.updated_at(), entity.updated_at());
    }

    #[test]
    fn test_input_entity() {
        let translator = TestTranslator;
        assert_eq!(translator.input_entity(), TypeId::of::<Uuid>());
    }

    #[test]
    fn test_output_entity() {
        let translator = TestTranslator;
        assert_eq!(translator.output_entity(), TypeId::of::<String>());
    }
}
