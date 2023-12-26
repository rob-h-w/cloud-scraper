use std::fmt::Debug;

use crate::domain::entity::Entity;
use crate::domain::source_identifier::SourceIdentifier;
use chrono::{DateTime, Utc};

pub(crate) trait Source<T>: Debug {
    fn source_identifier(&self) -> &SourceIdentifier;
    fn get(&mut self, since: &DateTime<Utc>) -> Result<Vec<Entity<T>>, Box<dyn std::error::Error>>;
}

#[cfg(test)]
pub(crate) mod tests {
    use chrono::{DateTime, Utc};

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
}
