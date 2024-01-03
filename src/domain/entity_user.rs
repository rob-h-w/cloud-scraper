use std::any::TypeId;

pub(crate) trait EntityUser {
    fn supported_entity_data() -> TypeId
    where
        Self: Sized;
}
