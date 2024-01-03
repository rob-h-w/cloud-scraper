use std::fmt::Debug;

use chrono::{DateTime, Utc};

use crate::domain::entity::Entity;
use crate::domain::entity_user::EntityUser;
use crate::domain::source_identifier::SourceIdentifier;

pub(crate) trait Source<DataType>: Debug + EntityUser
where
    DataType: Debug,
{
    fn identifier() -> &'static SourceIdentifier
    where
        Self: Sized;
    fn get(
        &mut self,
        since: &DateTime<Utc>,
    ) -> Result<Vec<Entity<DataType>>, Box<dyn std::error::Error>>;
}

#[cfg(test)]
pub(crate) mod tests {
    use std::any::TypeId;

    use chrono::{DateTime, Utc};
    use once_cell::sync::Lazy;

    use crate::domain::source::Source;

    use super::*;

    #[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
    pub(crate) struct TestSource {}

    impl TestSource {
        pub(crate) fn new() -> Self {
            Self {}
        }
    }

    impl EntityUser for TestSource {
        fn supported_entity_data() -> TypeId {
            TypeId::of::<String>()
        }
    }

    impl Source<String> for TestSource {
        fn identifier() -> &'static SourceIdentifier {
            static SOURCE_IDENTIFIER: Lazy<SourceIdentifier> =
                Lazy::new(|| SourceIdentifier::new("test"));
            &SOURCE_IDENTIFIER
        }

        fn get(
            &mut self,
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
        let source_name = "test";
        let mut source = TestSource::new();
        assert_eq!(
            TestSource::identifier(),
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
        assert_eq!(TestSource::supported_entity_data(), TypeId::of::<String>());
    }
}
