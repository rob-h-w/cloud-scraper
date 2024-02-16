use async_trait::async_trait;
use std::any::TypeId;
use std::error::Error;
use std::fmt::Debug;

use log::info;
use uuid::Uuid;

use crate::domain::entity::Entity;
use crate::domain::entity_consumer::EntityConsumer;
use crate::domain::entity_data::EntityData;
use crate::domain::entity_user::EntityUser;
use crate::domain::identifiable_sink::IdentifiableSink;
use crate::domain::sink::Sink;

#[derive(Debug)]
pub(crate) struct LogSink {}

impl LogSink {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl EntityUser for LogSink {
    fn supported_entity_data() -> Vec<TypeId>
    where
        Self: Sized,
    {
        vec![TypeId::of::<String>(), TypeId::of::<Uuid>()]
    }
}

impl IdentifiableSink for LogSink {
    const SINK_ID: &'static str = "log";
}

impl Sink<String> for LogSink {}

impl EntityData for String {}

#[async_trait]
impl EntityConsumer<String> for LogSink {
    async fn put(&self, entities: &[Entity<String>]) -> Result<(), Box<dyn Error>> {
        put(entities)
    }
}

impl Sink<Uuid> for LogSink {}

impl EntityData for Uuid {}

#[async_trait]
impl EntityConsumer<Uuid> for LogSink {
    async fn put(&self, entities: &[Entity<Uuid>]) -> Result<(), Box<dyn Error>> {
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

            let sink = LogSink::new();
            assert_eq!(LogSink::SINK_ID, "log");

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
