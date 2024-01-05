use std::any::TypeId;
use std::error::Error;
use std::fmt::Debug;

use log::info;
use once_cell::sync::Lazy;
use uuid::Uuid;

use crate::domain::entity::Entity;
use crate::domain::entity_consumer::EntityConsumer;
use crate::domain::entity_data::EntityData;
use crate::domain::entity_user::EntityUser;
use crate::domain::identifiable_sink::IdentifiableSink;
use crate::domain::sink::Sink;
use crate::domain::sink_identifier::SinkIdentifier;

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
    fn identifier() -> &'static SinkIdentifier {
        static SINK_IDENTIFIER: Lazy<SinkIdentifier> = Lazy::new(|| SinkIdentifier::new("log"));
        &SINK_IDENTIFIER
    }
}

impl Sink<String> for LogSink {}

impl EntityData for String {}

impl EntityConsumer<String> for LogSink {
    fn put(&self, entities: &[Entity<String>]) -> Result<(), Box<dyn Error>> {
        put(entities)
    }
}

impl Sink<Uuid> for LogSink {}

impl EntityData for Uuid {}

impl EntityConsumer<Uuid> for LogSink {
    fn put(&self, entities: &[Entity<Uuid>]) -> Result<(), Box<dyn Error>> {
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
    use crate::domain::source::tests::TestSource;
    use crate::tests::Logger;

    use super::*;

    #[test]
    fn test_log_sink() {
        Logger::init();
        Logger::use_in(|logger| {
            logger.reset();

            let sink = LogSink::new();
            assert_eq!(LogSink::identifier(), &SinkIdentifier::new("log"));

            let entities = vec![
                Entity::new_now::<TestSource>(Box::new("data 1".to_string()), "1"),
                Entity::new_now::<TestSource>(Box::new("data 2".to_string()), "2"),
            ];

            assert_eq!(logger.log_entries().len(), 0);
            sink.put(&entities).unwrap();

            let log_entries = logger.log_entries();

            println!("log entries: {:?}", log_entries);

            assert_eq!(log_entries.len(), 2);
        });
    }
}
