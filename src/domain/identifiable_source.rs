use crate::domain::entity_user::EntityUser;

pub(crate) trait IdentifiableSource: EntityUser {
    const SOURCE_ID: &'static str;

    fn this_identifier(&self) -> &'static str {
        Self::SOURCE_ID
    }
}
