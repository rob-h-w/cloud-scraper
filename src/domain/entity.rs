use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::entity_identifier::EntityIdentifier;
use crate::domain::source::Source;

#[derive(Debug, Deserialize, Hash, PartialEq, PartialOrd, Serialize)]
pub(crate) struct Entity<T> {
    created_at: DateTime<Utc>,
    data: Box<T>,
    id: EntityIdentifier,
    updated_at: DateTime<Utc>,
}

impl<T> Entity<T> {
    pub(crate) fn new(
        created_at: &DateTime<Utc>,
        data: Box<T>,
        id: &str,
        source: &dyn Source<T>,
        updated_at: &DateTime<Utc>,
    ) -> Self {
        Self {
            id: EntityIdentifier::new(id, source),
            created_at: created_at.clone(),
            updated_at: updated_at.clone(),
            data,
        }
    }

    pub(crate) fn new_now(data: Box<T>, id: &str, source: &dyn Source<T>) -> Self {
        Self::new(&Utc::now(), data, id, source, &Utc::now())
    }

    pub(crate) fn id(&self) -> &EntityIdentifier {
        &self.id
    }

    pub(crate) fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    pub(crate) fn updated_at(&self) -> &DateTime<Utc> {
        &self.updated_at
    }

    pub(crate) fn data(&self) -> &T {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use crate::domain::entity::Entity;
    use crate::domain::entity_identifier::EntityIdentifier;
    use crate::domain::source::tests::TestSource;

    fn assert_right_about_now(time: &DateTime<Utc>) {
        assert!(*time - Utc::now() < chrono::Duration::seconds(1));
    }

    #[test]
    fn test_entity_new() {
        let source = TestSource::new("test");
        let created_at = Utc::now() - chrono::Duration::days(1);
        let updated_at = Utc::now();
        let entity = Entity::new(
            &created_at,
            Box::new("data".to_string()),
            "1",
            &source,
            &updated_at,
        );
        assert_eq!(
            entity,
            Entity {
                created_at,
                data: Box::new("data".to_string()),
                id: EntityIdentifier::new("1", &source),
                updated_at,
            }
        );
    }

    #[test]
    fn test_entity_new_now() {
        let source = TestSource::new("test");
        let entity = Entity::new_now(Box::new("data".to_string()), "1", &source);
        let expected_entity = Entity::new(
            &entity.created_at,
            Box::new("data".to_string()),
            "1",
            &source,
            &entity.updated_at,
        );
        assert_eq!(entity, expected_entity);

        assert_right_about_now(entity.created_at());
        assert_right_about_now(entity.updated_at());
    }

    #[test]
    fn test_entity_id() {
        let source = TestSource::new("test");
        let entity = Entity::new_now(Box::new("data".to_string()), "1", &source);
        assert_eq!(entity.id(), &EntityIdentifier::new("1", &source));
    }

    #[test]
    fn test_entity_created_at() {
        let source = TestSource::new("test");
        let entity = Entity::new_now(Box::new("data".to_string()), "1", &source);
        assert_right_about_now(entity.created_at());
    }

    #[test]
    fn test_entity_updated_at() {
        let source = TestSource::new("test");
        let entity = Entity::new_now(Box::new("data".to_string()), "1", &source);
        assert_right_about_now(entity.updated_at());
    }

    #[test]
    fn test_entity_data() {
        let source = TestSource::new("test");
        let entity = Entity::new_now(Box::new("data".to_string()), "1", &source);
        assert_eq!(entity.data(), &"data".to_string());
    }
}
