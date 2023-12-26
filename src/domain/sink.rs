use crate::domain::entity::Entity;
use crate::domain::sink_identifier::SinkIdentifier;
use std::error::Error;

pub(crate) trait Sink<T> {
    fn sink_identifier(&self) -> &SinkIdentifier;
    fn put(&mut self, entities: Vec<Entity<T>>) -> Result<(), Box<dyn Error>>;
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::domain::entity::Entity;
    use crate::domain::sink::Sink;
    use crate::domain::sink_identifier::SinkIdentifier;
    use crate::domain::source::tests::TestSource;
    use std::error::Error;

    #[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
    pub(crate) struct TestSink {
        sink_identifier: SinkIdentifier,
    }

    impl TestSink {
        pub(crate) fn new(unique_name: &str) -> Self {
            Self {
                sink_identifier: SinkIdentifier::new(unique_name),
            }
        }
    }

    impl Sink<String> for TestSink {
        fn sink_identifier(&self) -> &SinkIdentifier {
            &self.sink_identifier
        }

        fn put(&mut self, entities: Vec<Entity<String>>) -> Result<(), Box<dyn Error>> {
            println!("putting entities: {:?}", entities);
            Ok(())
        }
    }

    #[test]
    fn test_dev_usability() {
        let source = TestSource::new("test");
        let sink_name = "test";
        let mut sink = TestSink::new(sink_name);
        assert_eq!(sink.sink_identifier(), &SinkIdentifier::new(sink_name));

        let entities = vec![
            Entity::new_now(Box::new("data 1".to_string()), "1", &source),
            Entity::new_now(Box::new("data 2".to_string()), "2", &source),
        ];
        sink.put(entities).unwrap();
    }
}
