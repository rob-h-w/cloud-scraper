use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::any::TypeId;

use crate::domain::entity::Entity;
use crate::domain::entity_data::EntityData;
use crate::domain::identifiable_source::IdentifiableSource;

#[async_trait]
pub(crate) trait Source<DataType>: IdentifiableSource + Send + Sync
where
    DataType: EntityData,
{
    async fn get(
        &self,
        since: &DateTime<Utc>,
    ) -> Result<Vec<Entity<DataType>>, Box<dyn std::error::Error>>;

    fn data_type(&self) -> TypeId {
        TypeId::of::<DataType>()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::any::TypeId;

    use crate::block_on;
    use crate::domain::entity_user::EntityUser;
    use chrono::{DateTime, Utc};

    use crate::domain::source::Source;

    use super::*;

    #[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
    pub(crate) struct TestSource {}

    impl TestSource {
        pub(crate) fn new() -> Self {
            Self {}
        }
    }

    impl IdentifiableSource for TestSource {
        const SOURCE_ID: &'static str = "test source";
    }

    impl EntityUser for TestSource {
        fn supported_entity_data() -> TypeId
        where
            Self: Sized,
        {
            TypeId::of::<String>()
        }
    }

    #[async_trait]
    impl Source<String> for TestSource {
        async fn get(
            &self,
            _since: &DateTime<Utc>,
        ) -> Result<Vec<Entity<String>>, Box<dyn std::error::Error>> {
            Ok(vec![
                Entity::new_now::<Self>(Box::new("data 1".to_string()), "1"),
                Entity::new_now::<Self>(Box::new("data 2".to_string()), "2"),
            ])
        }
    }

    #[test]
    fn test_dev_usability() {
        let source = TestSource::new();

        let since = Utc::now();
        let entities = block_on!(source.get(&since)).unwrap();
        assert_eq!(entities.len(), 2);
        assert_eq!(entities[0].data(), &"data 1".to_string());
        assert_eq!(entities[1].data(), &"data 2".to_string());
    }

    #[test]
    fn test_entity_user() {
        assert_eq!(TestSource::supported_entity_data(), TypeId::of::<String>());
    }
}
