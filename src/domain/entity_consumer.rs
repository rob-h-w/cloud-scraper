use std::error::Error;

use crate::domain::entity::Entity;
use crate::domain::entity_data::EntityData;

pub(crate) trait EntityConsumer<DataType>
where
    DataType: EntityData,
{
    fn put(&mut self, entities: &[Entity<DataType>]) -> Result<(), Box<dyn Error>>;
}
