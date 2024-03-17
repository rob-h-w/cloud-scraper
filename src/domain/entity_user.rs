use std::any::TypeId;
use std::fmt::Debug;

pub(crate) trait EntityUser: Debug {
    fn supported_entity_data() -> TypeId
    where
        Self: Sized;

    fn this_supported_entity_data(&self) -> TypeId
    where
        Self: Sized,
    {
        Self::supported_entity_data()
    }
}
