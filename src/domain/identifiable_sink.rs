use crate::domain::entity_user::EntityUser;

pub(crate) trait IdentifiableSink: EntityUser {
    const SINK_ID: &'static str;

    fn this_identifier(&self) -> &'static str {
        Self::SINK_ID
    }
}
