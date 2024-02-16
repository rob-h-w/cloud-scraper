use async_trait::async_trait;
use std::error::Error;

use crate::domain::entity::Entity;
use crate::domain::entity_data::EntityData;

#[async_trait]
pub(crate) trait EntityConsumer<DataType>
where
    DataType: EntityData,
{
    async fn put(&self, entities: &[Entity<DataType>]) -> Result<(), Box<dyn Error>>;
}
