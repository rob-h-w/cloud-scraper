use std::fmt::Debug;

use crate::domain::entity::Entity;
use crate::domain::source_identifier::SourceIdentifier;
use chrono::{DateTime, Utc};

pub(crate) trait Source<T>: Debug {
    fn source_identifier(&self) -> &SourceIdentifier;
    fn get(&self, since: &DateTime<Utc>) -> Vec<Entity<T>>;
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::domain::entity_identifier::EntityIdentifier;
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
                source_identifier: SourceIdentifier {
                    unique_name: unique_name.to_string(),
                },
            }
        }
    }

    impl Source<String> for TestSource {
        fn source_identifier(&self) -> &SourceIdentifier {
            &self.source_identifier
        }

        fn get(&self, _since: &DateTime<Utc>) -> Vec<Entity<String>> {
            let now = Utc::now();
            vec![
                Entity {
                    id: EntityIdentifier::new("1", self.source_identifier()),
                    created_at: now,
                    updated_at: now,
                    data: "data 1".parse().unwrap(),
                },
                Entity {
                    id: EntityIdentifier::new("2", self.source_identifier()),
                    created_at: now,
                    updated_at: now,
                    data: "data 2".parse().unwrap(),
                },
            ]
        }
    }
}
