use crate::domain::entity_user::EntityUser;
use crate::domain::sink_identifier::SinkIdentifier;

pub(crate) trait IdentifiableSink: EntityUser {
    fn identifier() -> &'static SinkIdentifier;
}
