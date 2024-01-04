use std::any::TypeId;
use std::fmt::Debug;

pub(crate) trait EntityUser: Debug {
    fn supported_entity_data() -> Vec<TypeId>
    where
        Self: Sized;
}
