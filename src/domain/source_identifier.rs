use std::fmt::Debug;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct SourceIdentifier {
    pub unique_name: String,
}
