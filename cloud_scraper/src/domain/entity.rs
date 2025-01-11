use std::fmt::Debug;

use crate::domain::entity_data::EntityData;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Hash, PartialEq, PartialOrd, Serialize)]
pub(crate) struct Entity<DataType>
where
    DataType: EntityData,
{
    created_at: DateTime<Utc>,
    data: DataType,
    id: String,
    updated_at: DateTime<Utc>,
}

impl<DataType> Entity<DataType>
where
    DataType: EntityData,
{
    pub(crate) fn new(
        created_at: DateTime<Utc>,
        data: DataType,
        id: &str,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            created_at: created_at.clone(),
            data,
            id: id.to_string(),
            updated_at,
        }
    }

    pub(crate) fn new_now(data: DataType, id: &str) -> Self {
        Self::new(Utc::now(), data, id, Utc::now())
    }

    #[cfg(test)]
    pub(crate) fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    #[cfg(test)]
    pub(crate) fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    #[cfg(test)]
    pub(crate) fn data(&self) -> &DataType {
        &self.data
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeDelta, Utc};

    use crate::domain::entity::Entity;

    fn assert_right_about_now(time: DateTime<Utc>) {
        assert!(time - Utc::now() < TimeDelta::try_seconds(1).unwrap());
    }

    #[test]
    fn test_entity_new_now() {
        let entity = Entity::new_now("data".to_string(), "1");
        let expected_entity = Entity::new(
            entity.created_at,
            "data".to_string(),
            "1",
            entity.updated_at,
        );
        assert_eq!(entity, expected_entity);

        assert_right_about_now(entity.created_at());
        assert_right_about_now(entity.updated_at());
    }

    #[test]
    fn test_entity_created_at() {
        let entity = Entity::new_now("data".to_string(), "1");
        assert_right_about_now(entity.created_at());
    }

    #[test]
    fn test_entity_updated_at() {
        let entity = Entity::new_now("data".to_string(), "1");
        assert_right_about_now(entity.updated_at());
    }

    #[test]
    fn test_entity_data() {
        let entity = Entity::new_now("data".to_string(), "1");
        assert_eq!(entity.data(), &"data".to_string());
    }
}
