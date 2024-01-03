use std::error::Error;
use std::fmt::Debug;

use log::info;
use once_cell::sync::Lazy;
use serde::Serialize;

use crate::domain::entity::Entity;
use crate::domain::sink::Sink;
use crate::domain::sink_identifier::SinkIdentifier;

#[derive(Debug)]
pub(crate) struct LogSink {}

impl LogSink {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl Sink for LogSink {
    fn identifier() -> &'static SinkIdentifier {
        static SINK_IDENTIFIER: Lazy<SinkIdentifier> = Lazy::new(|| SinkIdentifier::new("log"));
        &SINK_IDENTIFIER
    }

    fn put<DataType>(&mut self, entities: &Vec<Entity<DataType>>) -> Result<(), Box<dyn Error>>
    where
        DataType: Debug + Serialize,
    {
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

            let mut sink = LogSink::new();
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
