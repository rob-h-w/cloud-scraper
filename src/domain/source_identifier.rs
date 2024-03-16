use std::fmt::Debug;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) struct SourceIdentifier {
    unique_name: String,
}

impl SourceIdentifier {
    pub(crate) fn new(unique_name: &str) -> Self {
        Self {
            unique_name: unique_name.to_string(),
        }
    }

    pub(crate) fn unique_name(&self) -> &str {
        &self.unique_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_identifier_new() {
        let source_identifier = SourceIdentifier::new("test");
        assert_eq!(
            source_identifier,
            SourceIdentifier {
                unique_name: "test".to_string(),
            }
        );
    }

    #[test]
    fn test_source_identifier_unique_name() {
        let source_identifier = SourceIdentifier::new("test");
        assert_eq!(source_identifier.unique_name(), "test");
    }
}
