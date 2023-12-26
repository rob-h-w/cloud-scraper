use crate::domain::source::Source;
use std::hash::Hash;

use crate::domain::source_identifier::SourceIdentifier;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct EntityIdentifier {
    name: String,
    source_identifier: SourceIdentifier,
}

impl EntityIdentifier {
    pub(crate) fn new<T>(name: &str, source: &dyn Source<T>) -> Self {
        Self {
            name: name.to_string(),
            source_identifier: source.source_identifier().clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;

    use crate::domain::source::tests::TestSource;
    use crate::domain::source::Source;

    use super::*;

    #[test]
    fn test_entity_identifier_new() {
        let source = TestSource::new("test");
        let entity_identifier = EntityIdentifier::new("test", &source);
        assert_eq!(
            entity_identifier,
            EntityIdentifier {
                name: "test".to_string(),
                source_identifier: source.source_identifier().clone(),
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

        let source_1 = TestSource::new("test 1");
        let source_2 = TestSource::new("test 2");
        let [hash_1, hash_2] = source_1
            .get(&Utc::now())
            .iter()
            .map(|entity| hash(&entity.id))
            .collect::<Vec<u64>>()[..]
        else {
            panic!("Expected 2 entities");
        };
        assert_ne!(hash_1, hash_2);

        let hash_3 = hash(&EntityIdentifier::new("1", &source_1));
        assert_eq!(hash_1, hash_3);

        let hash_4 = hash(&EntityIdentifier::new("1", &source_2));
        assert_ne!(hash_1, hash_4);
    }
}
