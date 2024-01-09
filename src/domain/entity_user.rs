use std::any::TypeId;
use std::fmt::Debug;

pub(crate) trait EntityUser: Debug {
    fn supported_entity_data() -> Vec<TypeId>
    where
        Self: Sized;

    fn this_supports_entity_data(&self) -> Vec<TypeId>
    where
        Self: Sized,
    {
        Self::supported_entity_data()
    }
}
