use std::fmt::Debug;

use crate::domain::entity_data::EntityData;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::entity_identifier::EntityIdentifier;
use crate::domain::source::Source;

#[derive(Debug, Deserialize, Hash, PartialEq, PartialOrd, Serialize)]
pub(crate) struct Entity<DataType>
where
    DataType: EntityData,
{
    created_at: DateTime<Utc>,
    data: Box<DataType>,
    id: EntityIdentifier,
    updated_at: DateTime<Utc>,
}

impl<DataType> Entity<DataType>
where
    DataType: EntityData,
{
    pub(crate) fn new<SourceType>(
        created_at: &DateTime<Utc>,
        data: Box<DataType>,
        id: &str,
        updated_at: &DateTime<Utc>,
    ) -> Self
    where
        SourceType: Source<DataType>,
    {
        Self {
            id: EntityIdentifier::new::<SourceType, DataType>(id),
            created_at: created_at.clone(),
            updated_at: updated_at.clone(),
            data,
        }
    }

    pub(crate) fn new_now<SourceType>(data: Box<DataType>, id: &str) -> Self
    where
        SourceType: Source<DataType>,
    {
        Self::new::<SourceType>(&Utc::now(), data, id, &Utc::now())
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

    pub(crate) fn data(&self) -> &DataType {
        self.data.as_ref()
    }

    pub(crate) fn with_data<NewDataType>(&self, data: &NewDataType) -> Entity<NewDataType>
    where
        NewDataType: EntityData,
    {
        Entity {
            created_at: self.created_at.clone(),
            data: Box::new(data.clone()),
            id: self.id.clone(),
            updated_at: self.updated_at.clone(),
        }
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
        let created_at = Utc::now() - chrono::Duration::days(1);
        let updated_at = Utc::now();
        let entity =
            Entity::new::<TestSource>(&created_at, Box::new("data".to_string()), "1", &updated_at);
        assert_eq!(
            entity,
            Entity {
                created_at,
                data: Box::new("data".to_string()),
                id: EntityIdentifier::new::<TestSource, String>("1"),
                updated_at,
            }
        );
    }

    #[test]
    fn test_entity_new_now() {
        let entity = Entity::new_now::<TestSource>(Box::new("data".to_string()), "1");
        let expected_entity = Entity::new::<TestSource>(
            &entity.created_at,
            Box::new("data".to_string()),
            "1",
            &entity.updated_at,
        );
        assert_eq!(entity, expected_entity);

        assert_right_about_now(entity.created_at());
        assert_right_about_now(entity.updated_at());
    }

    #[test]
    fn test_entity_id() {
        let entity = Entity::new_now::<TestSource>(Box::new("data".to_string()), "1");
        assert_eq!(
            entity.id(),
            &EntityIdentifier::new::<TestSource, String>("1")
        );
    }

    #[test]
    fn test_entity_created_at() {
        let entity = Entity::new_now::<TestSource>(Box::new("data".to_string()), "1");
        assert_right_about_now(entity.created_at());
    }

    #[test]
    fn test_entity_updated_at() {
        let entity = Entity::new_now::<TestSource>(Box::new("data".to_string()), "1");
        assert_right_about_now(entity.updated_at());
    }

    #[test]
    fn test_entity_data() {
        let entity = Entity::new_now::<TestSource>(Box::new("data".to_string()), "1");
        assert_eq!(entity.data(), &"data".to_string());
    }
}
