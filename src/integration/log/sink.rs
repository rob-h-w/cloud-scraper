use std::error::Error;

use log::info;

use crate::domain::entity::Entity;
use crate::domain::sink::Sink;
use crate::domain::sink_identifier::SinkIdentifier;

pub(crate) struct LogSink {
    sink_identifier: SinkIdentifier,
}

impl LogSink {
    pub(crate) fn new() -> Self {
        Self {
            sink_identifier: SinkIdentifier::new("log"),
        }
    }
}

impl Sink<String> for LogSink {
    fn sink_identifier(&self) -> &SinkIdentifier {
        &self.sink_identifier
    }

    fn put(&mut self, entities: Vec<Entity<String>>) -> Result<(), Box<dyn Error>> {
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
            sink.put(entities).unwrap();

            let log_entries = logger.log_entries();

            println!("log entries: {:?}", log_entries);

            assert_eq!(log_entries.len(), 2);
        });
    }
}
