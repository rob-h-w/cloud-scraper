use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) struct SinkIdentifier {
    unique_name: String,
}

impl SinkIdentifier {
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
    fn test_sink_identifier_new() {
        let sink_identifier = SinkIdentifier::new("test");
        assert_eq!(
            sink_identifier,
            SinkIdentifier {
                unique_name: "test".to_string(),
            }
        );
    }

    #[test]
    fn test_sink_identifier_unique_name() {
        let sink_identifier = SinkIdentifier::new("test");
        assert_eq!(sink_identifier.unique_name(), "test");
    }
}
