use std::error::Error;
use std::fmt::Debug;

use crate::domain::entity::Entity;
use crate::domain::entity_user::EntityUser;
use crate::domain::sink_identifier::SinkIdentifier;

pub(crate) trait Sink<T>: Debug + EntityUser {
    fn sink_identifier(&self) -> &SinkIdentifier;
    fn put(&mut self, entities: &Vec<Entity<T>>) -> Result<(), Box<dyn Error>>;
}

#[cfg(test)]
pub(crate) mod tests {
    use std::any::TypeId;

    use crate::domain::sink::Sink;
    use crate::domain::source::tests::TestSource;

    use super::*;

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

    impl EntityUser for TestSink {
        fn supported_entity_data(&self) -> Vec<TypeId> {
            vec![TypeId::of::<String>()]
        }
    }

    impl Sink<String> for TestSink {
        fn sink_identifier(&self) -> &SinkIdentifier {
            &self.sink_identifier
        }

        fn put(&mut self, entities: &Vec<Entity<String>>) -> Result<(), Box<dyn Error>> {
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
        sink.put(&entities).unwrap();
    }

    #[test]
    fn test_sink_identifier() {
        let sink_name = "test";
        let sink = TestSink::new(sink_name);
        assert_eq!(sink.sink_identifier(), &SinkIdentifier::new(sink_name));
    }

    #[test]
    fn test_supported_entity_data() {
        let sink = TestSink::new("test");
        assert_eq!(sink.supported_entity_data(), vec![TypeId::of::<String>()]);
    }
}
