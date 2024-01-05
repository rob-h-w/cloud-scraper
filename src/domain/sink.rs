use crate::domain::entity_consumer::EntityConsumer;
use crate::domain::entity_data::EntityData;
use crate::domain::identifiable_sink::IdentifiableSink;

pub(crate) trait Sink<DataType>: IdentifiableSink + EntityConsumer<DataType>
where
    DataType: EntityData,
{
}

#[cfg(test)]
pub(crate) mod tests {
    use std::any::TypeId;
    use std::error::Error;

    use once_cell::sync::Lazy;

    use crate::domain::entity::Entity;
    use crate::domain::entity_consumer::EntityConsumer;
    use crate::domain::entity_user::EntityUser;
    use crate::domain::identifiable_sink::IdentifiableSink;
    use crate::domain::sink_identifier::SinkIdentifier;
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

    impl Sink<String> for TestSink {}

    impl EntityUser for TestSink {
        fn supported_entity_data() -> Vec<TypeId> {
            vec![TypeId::of::<String>()]
        }
    }

    impl IdentifiableSink for TestSink {
        fn identifier() -> &'static SinkIdentifier {
            static SINK_IDENTIFIER: Lazy<SinkIdentifier> =
                Lazy::new(|| SinkIdentifier::new("test"));
            &SINK_IDENTIFIER
        }
    }

    impl EntityConsumer<String> for TestSink {
        fn put(&self, entities: &[Entity<String>]) -> Result<(), Box<dyn Error>> {
            println!("putting entities: {:?}", entities);
            Ok(())
        }
    }

    #[test]
    fn test_dev_usability() {
        let sink_name = "test";
        let sink = TestSink::new(sink_name);
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
        assert_eq!(
            TestSink::supported_entity_data(),
            vec!(TypeId::of::<String>())
        );
    }
}
