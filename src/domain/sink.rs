use std::error::Error;
use std::fmt::Debug;

use serde::Serialize;

use crate::domain::entity::Entity;
use crate::domain::sink_identifier::SinkIdentifier;

pub(crate) trait Sink: Debug {
    fn identifier() -> &'static SinkIdentifier;
    fn put<DataType>(&mut self, entities: &Vec<Entity<DataType>>) -> Result<(), Box<dyn Error>>
    where
        DataType: Debug + Serialize;
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::domain::entity_user::EntityUser;
    use std::any::TypeId;

    use once_cell::sync::Lazy;

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
        fn supported_entity_data() -> TypeId {
            TypeId::of::<String>()
        }
    }

    impl Sink for TestSink {
        fn identifier() -> &'static SinkIdentifier {
            static SINK_IDENTIFIER: Lazy<SinkIdentifier> =
                Lazy::new(|| SinkIdentifier::new("test"));
            &SINK_IDENTIFIER
        }

        fn put<String: std::fmt::Debug>(
            &mut self,
            entities: &Vec<Entity<String>>,
        ) -> Result<(), Box<dyn Error>> {
            println!("putting entities: {:?}", entities);
            Ok(())
        }
    }

    #[test]
    fn test_dev_usability() {
        let sink_name = "test";
        let mut sink = TestSink::new(sink_name);
        assert_eq!(TestSink::identifier(), &SinkIdentifier::new(sink_name));

        let entities = vec![
            Entity::new_now::<TestSource>(Box::new("data 1".to_string()), "1"),
            Entity::new_now::<TestSource>(Box::new("data 2".to_string()), "2"),
        ];
        sink.put(&entities).unwrap();
    }

    #[test]
    fn test_sink_identifier() {
        assert_eq!(TestSink::identifier(), &SinkIdentifier::new("test"));
    }

    #[test]
    fn test_supported_entity_data() {
        assert_eq!(TestSink::supported_entity_data(), TypeId::of::<String>());
    }
}
