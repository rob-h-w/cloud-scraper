use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::core::cli::Cli;
use clap::Parser;
use serde::Deserialize;
use serde_yaml::Value;

use crate::domain::config::{Config as DomainConfig, PipelineConfig};
use crate::domain::source_identifier::SourceIdentifier;

const TLS_PORT: u16 = 443;
const DEFAULT_SITE_FOLDER: &str = ".site";

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    domain_config: Option<crate::domain::config::DomainConfig>,
    exit_after: Option<u64>,
    sinks: HashMap<String, Value>,
    sources: HashMap<String, Value>,
    pipelines: Vec<PipelineConfig>,
    port: Option<u16>,
    site_state_folder: Option<String>,
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
                config.merge_port(cli.port);
                config
            }
            None => Self {
                domain_config: None,
                exit_after: cli.exit_after,
                sinks: Self::sinks(),
                sources: Self::sources(),
                pipelines: Self::pipelines(),
                port: cli.port,
                site_state_folder: None,
            },
        })
    }

    #[cfg(test)]
    pub(crate) fn new_test() -> Arc<Self> {
        Arc::new(Self {
            domain_config: None,
            exit_after: None,
            sinks: Self::sinks(),
            sources: Self::sources(),
            pipelines: Self::pipelines(),
            port: None,
            site_state_folder: None,
        })
    }

    fn pipelines() -> Vec<PipelineConfig> {
        vec![PipelineConfig::new("log", "stub", None)]
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

    fn merge_port(&mut self, port: Option<u16>) {
        if let Some(p) = port {
            self.port = Some(p);
        }
    }
}

impl DomainConfig for Config {
    fn domain_config(&self) -> Option<&crate::domain::config::DomainConfig> {
        self.domain_config.as_ref()
    }

    fn exit_after(&self) -> Option<Duration> {
        self.exit_after.map(Duration::from_secs)
    }

    fn sink(&self, sink: &str) -> Option<&Value> {
        self.sinks.get(sink)
    }

    fn source(&self, source_identifier: &SourceIdentifier) -> Option<&Value> {
        self.sources.get(source_identifier.unique_name())
    }

    fn pipelines(&self) -> &Vec<PipelineConfig> {
        self.pipelines.as_ref()
    }

    fn port(&self) -> u16 {
        self.port.unwrap_or(TLS_PORT)
    }

    fn sink_names(&self) -> Vec<String> {
        self.sinks.keys().cloned().collect()
    }

    fn sink_configured(&self, name: &str) -> bool {
        self.sinks.contains_key(name)
    }

    fn site_folder(&self) -> &str {
        match self.site_state_folder {
            Some(ref folder) => folder.as_str(),
            None => DEFAULT_SITE_FOLDER,
        }
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
            domain_config: None,
            exit_after: None,
            sinks: Default::default(),
            sources: Default::default(),
            pipelines: vec![],
            port: None,
            site_state_folder: None,
        };

        assert_eq!(config.sink(TestSink::SINK_ID), None);
        assert_eq!(config.source(TestSource::identifier()), None);
        assert!(config.pipelines().is_empty());
        assert_eq!(config.port(), TLS_PORT);
        assert!(config.sanity_check().is_ok());
    }
}
