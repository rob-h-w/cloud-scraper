use std::any::TypeId;

pub(crate) trait EntityUser {
    fn supported_entity_data(&self) -> Vec<TypeId>;
}
