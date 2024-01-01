use crate::domain::config::{Config as DomainConfig, PipelineConfig};
use crate::domain::sink::Sink;
use crate::domain::source::Source;
use serde::Deserialize;
use serde_yaml::Value;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    sinks: HashMap<String, Value>,
    sources: HashMap<String, Value>,
    pipelines: Vec<PipelineConfig>,
}

impl Config {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self {
            sinks: Default::default(),
            sources: Default::default(),
            pipelines: vec![],
        })
    }
}

impl DomainConfig for Config {
    fn sink<T>(&self, sink: &impl Sink<T>) -> Option<&Value> {
        self.sinks.get(sink.sink_identifier().unique_name())
    }

    fn source<T>(&self, source: &impl Source<T>) -> Option<&Value> {
        self.sources.get(source.source_identifier().unique_name())
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
    use super::*;
    use crate::domain::sink::tests::TestSink;
    use crate::domain::source::tests::TestSource;

    #[test]
    fn test_instantiate() {
        let config = Config {
            sinks: Default::default(),
            sources: Default::default(),
            pipelines: vec![],
        };

        assert_eq!(config.sink(&TestSink::new("test")), None);
        assert_eq!(config.source(&TestSource::new("test")), None);
        assert!(config.pipelines().is_empty());
        assert!(config.sanity_check().is_ok());
    }
}
