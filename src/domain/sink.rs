use crate::domain::entity::Entity;
use crate::domain::sink_identifier::SinkIdentifier;
use std::error::Error;

pub(crate) trait Sink<T> {
    fn sink_identifier(&self) -> &SinkIdentifier;
    fn put(&self, entities: Vec<Entity<T>>) -> Result<(), Box<dyn Error>>;
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::domain::entity::Entity;
    use crate::domain::entity_identifier::EntityIdentifier;
    use crate::domain::sink::Sink;
    use crate::domain::sink_identifier::SinkIdentifier;
    use crate::domain::source::tests::TestSource;
    use crate::domain::source::Source;
    use std::error::Error;

    #[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
    pub(crate) struct TestSink {
        sink_identifier: SinkIdentifier,
    }

    impl TestSink {
        pub(crate) fn new(unique_name: &str) -> Self {
            Self {
                sink_identifier: SinkIdentifier {
                    unique_name: unique_name.to_string(),
                },
            }
        }
    }

    impl Sink<String> for TestSink {
        fn sink_identifier(&self) -> &SinkIdentifier {
            &self.sink_identifier
        }

        fn put(&self, entities: Vec<Entity<String>>) -> Result<(), Box<dyn Error>> {
            println!("putting entities: {:?}", entities);
            Ok(())
        }
    }

    #[test]
    fn test_dev_usability() {
        let source = TestSource::new("test");
        let sink_name = "test";
        let sink = TestSink::new(sink_name);
        assert_eq!(
            sink.sink_identifier(),
            &SinkIdentifier {
                unique_name: sink_name.to_string(),
            }
        );

        let entities = vec![
            Entity {
                id: EntityIdentifier::new("1", source.source_identifier()),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                data: "data 1".to_string(),
            },
            Entity {
                id: EntityIdentifier::new("2", source.source_identifier()),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                data: "data 2".to_string(),
            },
        ];
        sink.put(entities).unwrap();
    }
}
