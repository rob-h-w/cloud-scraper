use std::fmt::Debug;
use std::hash::Hash;

use serde::{Deserialize, Serialize};

use crate::domain::entity_data::EntityData;
use crate::domain::source::Source;
use crate::domain::source_identifier::SourceIdentifier;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) struct EntityIdentifier {
    name: String,
    source_identifier: SourceIdentifier,
}

impl EntityIdentifier {
    pub(crate) fn new<SourceType, DataType>(name: &str) -> Self
    where
        SourceType: Source<DataType>,
        DataType: EntityData,
    {
        Self {
            name: name.to_string(),
            source_identifier: SourceType::identifier().clone(),
        }
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn source_identifier(&self) -> &SourceIdentifier {
        &self.source_identifier
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;

    use chrono::{DateTime, Utc};
    use once_cell::sync::Lazy;
    use uuid::Uuid;

    use crate::domain::entity::Entity;
    use crate::domain::entity_user::EntityUser;
    use crate::domain::source::tests::TestSource;
    use crate::domain::source::Source;

    use super::*;

    #[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
    struct TestSource2;

    impl EntityUser for TestSource2 {
        fn supported_entity_data() -> Vec<TypeId>
        where
            Self: Sized,
        {
            vec![TypeId::of::<Uuid>()]
        }
    }

    impl Source<Uuid> for TestSource2 {
        fn identifier() -> &'static SourceIdentifier {
            static SOURCE_IDENTIFIER: Lazy<SourceIdentifier> =
                Lazy::new(|| SourceIdentifier::new("test2"));
            &SOURCE_IDENTIFIER
        }

        fn get(
            &mut self,
            _since: &DateTime<Utc>,
        ) -> Result<Vec<Entity<Uuid>>, Box<dyn std::error::Error>> {
            Ok(vec![
                Entity::new_now::<Self>(Box::new(Uuid::new_v4()), "1"),
                Entity::new_now::<Self>(Box::new(Uuid::new_v4()), "2"),
            ])
        }
    }

    #[test]
    fn test_entity_identifier_new() {
        let entity_identifier = EntityIdentifier::new::<TestSource, String>("test");
        assert_eq!(
            entity_identifier,
            EntityIdentifier {
                name: "test".to_string(),
                source_identifier: TestSource::identifier().clone(),
            }
        );
    }

    #[test]
    fn test_entity_identifier_hash() {
        fn hash(entity_identifier: &EntityIdentifier) -> u64 {
            let mut hasher = DefaultHasher::new();
            entity_identifier.hash(&mut hasher);
            hasher.finish()
        }

        let mut source_1 = TestSource::new();
        let [hash_1, hash_2] = source_1
            .get(&Utc::now())
            .unwrap()
            .iter()
            .map(|entity| hash(&entity.id()))
            .collect::<Vec<u64>>()[..]
        else {
            panic!("Expected 2 entities");
        };
        assert_ne!(hash_1, hash_2);

        let hash_3 = hash(&EntityIdentifier::new::<TestSource, String>("1"));
        assert_eq!(hash_1, hash_3);

        let hash_4 = hash(&EntityIdentifier::new::<TestSource2, Uuid>("1"));
        assert_ne!(hash_1, hash_4);
    }

    #[test]
    fn test_entity_identifier_name() {
        let entity_identifier = EntityIdentifier::new::<TestSource, String>("test");
        assert_eq!(entity_identifier.name(), "test");
    }

    #[test]
    fn test_entity_identifier_source_identifier() {
        let entity_identifier = EntityIdentifier::new::<TestSource, String>("test");
        assert_eq!(
            entity_identifier.source_identifier(),
            TestSource::identifier()
        );
    }
}
