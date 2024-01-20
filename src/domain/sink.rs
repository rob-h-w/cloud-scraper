use crate::domain::entity_consumer::EntityConsumer;
use crate::domain::entity_data::EntityData;
use crate::domain::identifiable_sink::IdentifiableSink;

pub(crate) trait Sink<DataType>: IdentifiableSink + EntityConsumer<DataType> + Sync
where
    DataType: EntityData,
{
}

#[cfg(test)]
pub(crate) mod tests {
    use std::any::TypeId;
    use std::error::Error;

    use crate::domain::entity::Entity;
    use crate::domain::entity_consumer::EntityConsumer;
    use crate::domain::entity_user::EntityUser;
    use crate::domain::identifiable_sink::IdentifiableSink;
    use crate::domain::source::tests::TestSource;

    use super::*;

    #[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
    pub(crate) struct TestSink {}

    impl Sink<String> for TestSink {}

    impl EntityUser for TestSink {
        fn supported_entity_data() -> Vec<TypeId> {
            vec![TypeId::of::<String>()]
        }
    }

    impl IdentifiableSink for TestSink {
        const SINK_ID: &'static str = "test";
    }

    impl EntityConsumer<String> for TestSink {
        fn put(&self, entities: &[Entity<String>]) -> Result<(), Box<dyn Error>> {
            println!("putting entities: {:?}", entities);
            Ok(())
        }
    }

    #[test]
    fn test_dev_usability() {
        let sink = TestSink {};
        assert_eq!(TestSink::SINK_ID, "test");

        let entities = vec![
            Entity::new_now::<TestSource>(Box::new("data 1".to_string()), "1"),
            Entity::new_now::<TestSource>(Box::new("data 2".to_string()), "2"),
        ];
        sink.put(&entities).unwrap();
    }

    #[test]
    fn test_sink_identifier() {
        assert_eq!(TestSink::SINK_ID, "test");
    }

    #[test]
    fn test_supported_entity_data() {
        assert_eq!(
            TestSink::supported_entity_data(),
            vec!(TypeId::of::<String>())
        );
    }
}
