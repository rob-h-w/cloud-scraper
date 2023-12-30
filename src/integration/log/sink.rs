use std::any::TypeId;
use std::error::Error;
use std::fmt::Debug;
use std::marker::PhantomData;

use log::info;
use serde::Serialize;

use crate::domain::entity::Entity;
use crate::domain::entity_user::EntityUser;
use crate::domain::sink::Sink;
use crate::domain::sink_identifier::SinkIdentifier;

#[derive(Debug)]
pub(crate) struct LogSink<T>
where
    T: Debug,
{
    sink_identifier: SinkIdentifier,
    phantom: PhantomData<T>,
}

impl<T> LogSink<T>
where
    T: Debug + 'static,
{
    pub(crate) fn new() -> Self {
        Self {
            sink_identifier: SinkIdentifier::new("log"),
            phantom: Default::default(),
        }
    }
}

impl<T> EntityUser for LogSink<T>
where
    T: Debug + 'static,
{
    fn supported_entity_data(&self) -> Vec<TypeId> {
        vec![TypeId::of::<T>()]
    }
}

impl<T> Sink<T> for LogSink<T>
where
    T: Debug + Serialize + 'static,
{
    fn sink_identifier(&self) -> &SinkIdentifier {
        &self.sink_identifier
    }

    fn put(&mut self, entities: &Vec<Entity<T>>) -> Result<(), Box<dyn Error>> {
        entities.iter().for_each(|entity| {
            info!("{}", serde_yaml::to_string(&entity).unwrap());
        });
        Ok(())
    }
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

            let source: TestSource = TestSource::new("test");
            let mut sink = LogSink::new();
            assert_eq!(sink.sink_identifier(), &SinkIdentifier::new("log"));

            let entities = vec![
                Entity::new_now(Box::new("data 1".to_string()), "1", &source),
                Entity::new_now(Box::new("data 2".to_string()), "2", &source),
            ];

            assert_eq!(logger.log_entries().len(), 0);
            sink.put(&entities).unwrap();

            let log_entries = logger.log_entries();

            println!("log entries: {:?}", log_entries);

            assert_eq!(log_entries.len(), 2);
        });
    }
}
