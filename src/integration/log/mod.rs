use async_trait::async_trait;
use std::any::TypeId;
use std::error::Error;
use std::fmt::Debug;

use log::info;

use crate::domain::entity::Entity;
use crate::domain::entity_data::EntityData;
use crate::domain::entity_user::EntityUser;
use crate::domain::identifiable_sink::IdentifiableSink;
use crate::domain::sink::Sink as SinkTrait;

#[derive(Debug)]
pub(crate) struct Sink;

impl Sink {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl EntityUser for Sink {
    fn supported_entity_data() -> Vec<TypeId>
    where
        Self: Sized,
    {
        vec![TypeId::of::<String>()]
    }
}

impl IdentifiableSink for Sink {
    const SINK_ID: &'static str = "log";
}

#[async_trait]
impl SinkTrait<String> for Sink {
    async fn put(&self, entities: &[Entity<String>]) -> Result<(), Box<dyn Error>> {
        put(entities)
    }
}

fn put<T>(entities: &[Entity<T>]) -> Result<(), Box<dyn Error>>
where
    T: EntityData,
{
    entities.iter().for_each(|entity| {
        info!("{}", serde_yaml::to_string(&entity).unwrap());
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::block_on;
    use crate::domain::source::tests::TestSource;
    use crate::tests::Logger;

    use super::*;

    #[test]
    fn test_log_sink() {
        Logger::init();
        Logger::use_in(|logger| {
            logger.reset();

            let sink = Sink::new();
            assert_eq!(Sink::SINK_ID, "log");

            let entities = vec![
                Entity::new_now::<TestSource>(Box::new("data 1".to_string()), "1"),
                Entity::new_now::<TestSource>(Box::new("data 2".to_string()), "2"),
            ];

            assert_eq!(logger.log_entries().len(), 0);
            block_on!(sink.put(&entities)).unwrap();

            let log_entries = logger.log_entries();

            println!("log entries: {:?}", log_entries);

            assert_eq!(log_entries.len(), 2);
        });
    }
}
