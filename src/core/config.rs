use std::collections::HashMap;
use std::rc::Rc;

use serde::Deserialize;
use serde_yaml::Value;

use crate::domain::config::{Config as DomainConfig, PipelineConfig};
use crate::domain::sink_identifier::SinkIdentifier;
use crate::domain::source_identifier::SourceIdentifier;

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    sinks: HashMap<String, Value>,
    sources: HashMap<String, Value>,
    pipelines: Vec<PipelineConfig>,
}

impl Config {
    pub(crate) fn new() -> Rc<Self> {
        let mut sources = HashMap::new();
        sources.insert("stub".to_string(), Value::Null);

        let mut sinks = HashMap::new();
        sinks.insert("log".to_string(), Value::Null);

        Rc::new(Self {
            sinks,
            sources,
            pipelines: vec![],
        })
    }
}

impl DomainConfig for Config {
    fn sink(&self, sink: &SinkIdentifier) -> Option<&Value> {
        self.sinks.get(sink.unique_name())
    }

    fn source(&self, source_identifier: &SourceIdentifier) -> Option<&Value> {
        self.sources.get(source_identifier.unique_name())
    }

    fn pipelines(&self) -> &Vec<PipelineConfig> {
        self.pipelines.as_ref()
    }

    fn sink_configured(&self, name: &str) -> bool {
        self.sinks.contains_key(name)
    }

    fn source_configured(&self, name: &str) -> bool {
        self.sources.contains_key(name)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::identifiable_sink::IdentifiableSink;
    use crate::domain::sink::tests::TestSink;
    use crate::domain::source::tests::TestSource;
    use crate::domain::source::Source;

    use super::*;

    #[test]
    fn test_instantiate() {
        let config = Config {
            sinks: Default::default(),
            sources: Default::default(),
            pipelines: vec![],
        };

        assert_eq!(config.sink(TestSink::identifier()), None);
        assert_eq!(config.source(TestSource::identifier()), None);
        assert!(config.pipelines().is_empty());
        assert!(config.sanity_check().is_ok());
    }
}
