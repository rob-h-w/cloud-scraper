use std::any::TypeId;
use std::cmp::min;
use std::error::Error;
use std::fmt::Debug;

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::domain::entity::Entity;
use crate::domain::entity_user::EntityUser;
use crate::domain::source::Source;
use crate::domain::source_identifier::SourceIdentifier;

#[derive(Debug)]
pub(crate) struct StubSource {
    source_identifier: SourceIdentifier,
}

impl StubSource {
    pub(crate) fn new() -> Self {
        Self {
            source_identifier: SourceIdentifier::new("stub"),
        }
    }
}

impl EntityUser for StubSource {
    fn supported_entity_data(&self) -> Vec<TypeId> {
        vec![TypeId::of::<Uuid>()]
    }
}

impl Source<Uuid> for StubSource {
    fn source_identifier(&self) -> &SourceIdentifier {
        &self.source_identifier
    }

    fn get(&mut self, since: &DateTime<Utc>) -> Result<Vec<Entity<Uuid>>, Box<dyn Error>> {
        let now = Utc::now();
        let diff = now - *since;

        if diff.num_seconds() < 1 || now < *since {
            return Ok(vec![]);
        }

        let results = (0..diff.num_seconds())
            .map(|i| {
                let created = *since + Duration::seconds(i);
                let updated = min(*since + Duration::seconds(i + 1), now);
                Entity::new(
                    &created,
                    Box::new(Uuid::new_v4()),
                    (*since + Duration::seconds(i))
                        .to_rfc3339()
                        .to_string()
                        .as_str(),
                    self,
                    &updated,
                )
            })
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use crate::domain::source::Source;

    use super::*;

    #[test]
    fn test_stub_source_new() {
        let source = StubSource::new();
        assert_eq!(source.source_identifier(), &SourceIdentifier::new("stub"));
    }

    #[test]
    fn test_stub_source_get() {
        let mut source = StubSource::new();
        let now = Utc::now();
        let since = now - Duration::seconds(1);
        let entities = source.get(&since).unwrap();
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].created_at(), &since);
        assert_eq!(entities[0].updated_at(), &(since + Duration::seconds(1)));

        let since = now + Duration::seconds(2);
        assert_eq!(source.get(&since).unwrap().len(), 0);

        let since = now - Duration::seconds(2);
        let entities = source.get(&since).unwrap();
        assert_eq!(entities.len(), 2);
        let last = &entities[1];
        assert_eq!(last.created_at(), &(now - Duration::seconds(1)));
        assert_eq!(last.updated_at(), &now);
    }

    #[test]
    fn test_stub_source_get_empty() {
        let mut source = StubSource::new();
        let since = Utc::now();
        let entities = source.get(&since).unwrap();
        assert_eq!(entities.len(), 0);
    }
}
