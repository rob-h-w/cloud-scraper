use async_trait::async_trait;
use std::any::TypeId;
use std::cell::RefCell;
use std::cmp::{max, min};
use std::error::Error;
use std::fmt::Debug;
use std::sync::Mutex;

use chrono::{DateTime, TimeDelta, Utc};
use uuid::Uuid;

use crate::domain::entity::Entity;
use crate::domain::entity_user::EntityUser;
use crate::domain::identifiable_source::IdentifiableSource;
use crate::domain::source::Source;

#[derive(Debug)]
pub(crate) struct StubSource {
    last: Mutex<RefCell<Option<DateTime<Utc>>>>,
}

impl StubSource {
    pub(crate) fn new() -> Self {
        Self {
            last: Mutex::new(RefCell::new(None)),
        }
    }
}

impl EntityUser for StubSource {
    fn supported_entity_data() -> TypeId {
        TypeId::of::<Uuid>()
    }
}

impl IdentifiableSource for StubSource {
    const SOURCE_ID: &'static str = "stub";
}

#[async_trait]
impl Source<Uuid> for StubSource {
    async fn get(&self, since: &DateTime<Utc>) -> Result<Vec<Entity<Uuid>>, Box<dyn Error>> {
        let cell = self.last.lock().unwrap();
        let prior_state = cell.borrow().is_some();
        let now = Utc::now();
        let last = min(cell.borrow().unwrap_or(now), *since);
        let diff = now - last;

        if prior_state && diff.num_seconds() < 1 {
            return Ok(vec![]);
        }

        cell.replace(Some(now));

        let results = (0..diff.num_seconds())
            .map(|i| {
                let created = *since + TimeDelta::try_seconds(i).unwrap();
                let updated = max(*since + TimeDelta::try_seconds(i + 1).unwrap(), last);
                Entity::new::<Self>(
                    &created,
                    Box::new(Uuid::new_v4()),
                    format!("uuid at {}", created.to_rfc3339().to_string().as_str()).as_str(),
                    &updated,
                )
            })
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use crate::block_on;
    use chrono::{TimeDelta, Utc};

    use crate::domain::source::Source;

    use super::*;

    #[test]
    fn test_stub_source_get() {
        let source = StubSource::new();
        let now = Utc::now();
        let since = now - TimeDelta::try_seconds(1).unwrap();
        let entities = block_on!(source.get(&since)).unwrap();
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].created_at(), &since);
        assert_eq!(
            entities[0].updated_at(),
            &(since + TimeDelta::try_seconds(1).unwrap())
        );

        let since = now + TimeDelta::try_seconds(2).unwrap();
        assert_eq!(block_on!(source.get(&since)).unwrap().len(), 0);

        let since = now - TimeDelta::try_seconds(2).unwrap();
        let entities = block_on!(source.get(&since)).unwrap();
        assert_eq!(entities.len(), 2);
        let last = &entities[1];
        assert_eq!(
            last.created_at(),
            &(now - TimeDelta::try_seconds(1).unwrap())
        );
        assert_eq!(last.updated_at(), &now);
    }

    #[test]
    fn test_stub_source_get_empty() {
        let source = StubSource::new();
        let since = Utc::now();
        let entities = block_on!(source.get(&since)).unwrap();
        assert_eq!(entities.len(), 0);
    }
}
