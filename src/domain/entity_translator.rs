use std::fmt::Debug;

use crate::domain::entity::Entity;
use crate::domain::entity_data::EntityData;

pub(crate) trait EntityTranslator<FromDataType, ToDataType>:
    Clone + Debug + Send + Sync + 'static
where
    FromDataType: EntityData,
    ToDataType: EntityData,
{
    fn translate(&self, entity: &Entity<FromDataType>) -> Entity<ToDataType>;
}

#[cfg(test)]
pub(crate) mod tests {
    use uuid::Uuid;

    use crate::integration::stub::source::StubSource;

    use super::*;

    #[derive(Clone, Debug)]
    pub(crate) struct TestTranslator;

    impl EntityTranslator<Uuid, String> for TestTranslator {
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
}
