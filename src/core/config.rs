use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::core::cli::Cli;
use clap::Parser;
use serde::Deserialize;
use serde_yaml::Value;

use crate::domain::config::{Config as DomainConfig, PipelineConfig};

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    exit_after: Option<u64>,
    sinks: HashMap<String, Value>,
    sources: HashMap<String, Value>,
    pipelines: Vec<PipelineConfig>,
}

impl Config {
    pub(crate) fn new() -> Arc<Self> {
        let cli = Cli::parse();
        Arc::new(match cli.config {
            Some(config_file) => {
                let config_file =
                    std::fs::read_to_string(config_file).expect("Could not open $config_file");
                let mut config: Config =
                    serde_yaml::from_str(&config_file).expect("Could not parse config");
                config.merge_exit_after(cli.exit_after);
                config
            }
            None => Self {
                exit_after: cli.exit_after,
                sinks: Self::sinks(),
                sources: Self::sources(),
                pipelines: Self::pipelines(),
            },
        })
    }

    #[cfg(test)]
    pub(crate) fn new_test() -> Arc<Self> {
        Arc::new(Self {
            exit_after: None,
            sinks: Self::sinks(),
            sources: Self::sources(),
            pipelines: Self::pipelines(),
        })
    }

    fn pipelines() -> Vec<PipelineConfig> {
        vec![PipelineConfig::new("log", "stub")]
    }

    fn sinks() -> HashMap<String, Value> {
        let mut sinks = HashMap::new();
        sinks.insert("log".to_string(), Value::Null);
        sinks
    }

    fn sources() -> HashMap<String, Value> {
        let mut sources = HashMap::new();
        sources.insert("stub".to_string(), Value::Null);
        sources
    }

    fn merge_exit_after(&mut self, exit_after: Option<u64>) {
        if exit_after.is_some() {
            self.exit_after = exit_after;
        }
    }
}

impl DomainConfig for Config {
    fn exit_after(&self) -> Option<Duration> {
        self.exit_after.map(Duration::from_secs)
    }

    fn sink(&self, sink: &str) -> Option<&Value> {
        self.sinks.get(sink)
    }

    fn source(&self, source_identifier: &str) -> Option<&Value> {
        self.sources.get(source_identifier)
    }

    fn pipelines(&self) -> &Vec<PipelineConfig> {
        self.pipelines.as_ref()
    }

    fn sink_names(&self) -> Vec<String> {
        self.sinks.keys().cloned().collect()
    }

    fn source_names(&self) -> Vec<String> {
        self.sources.keys().cloned().collect()
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
    use crate::domain::identifiable_source::IdentifiableSource;
    use crate::domain::sink::tests::TestSink;
    use crate::domain::source::tests::TestSource;

    use super::*;

    #[test]
    fn test_instantiate() {
        let config = Config {
            exit_after: None,
            sinks: Default::default(),
            sources: Default::default(),
            pipelines: vec![],
        };

        assert_eq!(config.sink(TestSink::SINK_ID), None);
        assert_eq!(config.source(TestSource::SOURCE_ID), None);
        assert!(config.pipelines().is_empty());
        assert!(config.sanity_check().is_ok());
    }
}
