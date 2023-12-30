use std::fmt::Debug;

use crate::domain::entity::Entity;
use crate::domain::entity_user::EntityUser;
use crate::domain::source_identifier::SourceIdentifier;
use chrono::{DateTime, Utc};

pub(crate) trait Source<T>: Debug + EntityUser {
    fn source_identifier(&self) -> &SourceIdentifier;
    fn get(&mut self, since: &DateTime<Utc>) -> Result<Vec<Entity<T>>, Box<dyn std::error::Error>>;
}

#[cfg(test)]
pub(crate) mod tests {
    use chrono::{DateTime, Utc};
    use std::any::TypeId;

    use crate::domain::source::Source;

    use super::*;

    #[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
    pub(crate) struct TestSource {
        source_identifier: SourceIdentifier,
    }

    impl TestSource {
        pub(crate) fn new(unique_name: &str) -> Self {
            Self {
                source_identifier: SourceIdentifier::new(unique_name),
            }
        }
    }

    impl EntityUser for TestSource {
        fn supported_entity_data(&self) -> Vec<TypeId> {
            vec![TypeId::of::<String>()]
        }
    }

    impl Source<String> for TestSource {
        fn source_identifier(&self) -> &SourceIdentifier {
            &self.source_identifier
        }

        fn get(
            &mut self,
            _since: &DateTime<Utc>,
        ) -> Result<Vec<Entity<String>>, Box<dyn std::error::Error>> {
            Ok(vec![
                Entity::new_now(Box::new("data 1".to_string()), "1", self),
                Entity::new_now(Box::new("data 2".to_string()), "2", self),
            ])
        }
    }

    #[test]
    fn test_dev_usability() {
        let source_name = "test";
        let mut source = TestSource::new(source_name);
        assert_eq!(
            source.source_identifier(),
            &SourceIdentifier::new(source_name)
        );

        let since = Utc::now();
        let entities = source.get(&since).unwrap();
        assert_eq!(entities.len(), 2);
        assert_eq!(entities[0].data(), &"data 1".to_string());
        assert_eq!(entities[1].data(), &"data 2".to_string());
    }

    #[test]
    fn test_entity_user() {
        let source = TestSource::new("test");

        assert_eq!(source.supported_entity_data(), vec![TypeId::of::<String>()]);
    }
}
