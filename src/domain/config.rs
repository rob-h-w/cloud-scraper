use std::vec;

use serde::Deserialize;
use serde_yaml::Value;

use crate::domain::sink_identifier::SinkIdentifier;
use crate::domain::source_identifier::SourceIdentifier;

pub(crate) trait Config {
    fn sink(&self, sink_identifier: &SinkIdentifier) -> Option<&Value>;
    fn source(&self, source_identifier: &SourceIdentifier) -> Option<&Value>;
    fn pipelines(&self) -> &Vec<PipelineConfig>;

    fn sink_configured(&self, name: &str) -> bool;
    fn source_configured(&self, name: &str) -> bool;

    fn sanity_check(&self) -> Result<(), String> {
        let mut errors = vec![];
        for pipeline in self.pipelines() {
            if !self.sink_configured(&pipeline.sink) {
                errors.push(format!("Sink '{}' not configured", pipeline.sink));
            }
            if !self.source_configured(&pipeline.source) {
                errors.push(format!("Source '{}' not found", pipeline.source));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join(", "))
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct PipelineConfig {
    source: String,
    sink: String,
    translator: Option<TranslatorConfig>,
}

#[derive(Debug, Deserialize)]
struct TranslatorConfig {
    from: String,
    to: String,
}

#[cfg(test)]
pub(crate) mod tests {
    use log::info;
    use serde_yaml::Mapping;

    use crate::domain::identifiable_sink::IdentifiableSink;
    use crate::domain::sink::tests::TestSink;
    use crate::domain::source::tests::TestSource;
    use crate::domain::source::Source;
    use crate::tests::Logger;

    use super::*;

    #[test]
    fn test_config_sink() {
        Logger::init();
        Logger::use_in(|logger| {
            logger.reset();
            let config = TestConfig::new(None);
            let sink_config = config.sink(TestSink::identifier()).unwrap();
            assert_eq!(sink_config, &Value::Mapping(Mapping::new()));

            let log_entries = logger.log_entries();
            assert_eq!(log_entries.len(), 1);
            let log_line = &log_entries[0];
            assert_eq!(log_line.level(), log::Level::Info);
            assert!(log_line.args().contains(
                "sink ID: SinkIdentifier { unique_name: \
            \"test\" }"
            ));
        });
    }

    #[test]
    fn test_config_source() {
        Logger::init();
        Logger::use_in(|logger| {
            logger.reset();
            let config = TestConfig::new(None);
            let source_config = config.source(TestSource::identifier()).unwrap();
            assert_eq!(source_config, &Value::Mapping(Mapping::new()));

            let log_entries = logger.log_entries();
            assert_eq!(log_entries.len(), 1);
            let log_line = &log_entries[0];
            assert_eq!(log_line.level(), log::Level::Info);
            assert!(log_line.args().contains(
                "source ID: SourceIdentifier { unique_name: \
            \"test\" }"
            ));
        });
    }

    #[test]
    fn test_config_pipelines() {
        let config = TestConfig::new(None);
        let pipelines = config.pipelines();
        assert_eq!(pipelines.len(), 1);

        let pipeline = &pipelines[0];
        assert_eq!(pipeline.source, "test");
        assert_eq!(pipeline.sink, "test");
        assert!(pipeline.translator.is_some());

        let translator = pipeline.translator.as_ref().unwrap();
        assert_eq!(translator.from, "Uuid");
        assert_eq!(translator.to, "String");
    }

    #[test]
    fn test_config_sanity_check() {
        let config = TestConfig::new(None);
        assert!(config.sanity_check().is_ok());

        let bad_config = TestConfig::new(Some(PipelineConfig {
            source: "test".to_string(),
            sink: "test2".to_string(),
            translator: None,
        }));
        assert!(bad_config.sanity_check().is_err());
    }

    pub(crate) struct TestConfig {
        value: Value,
        pipelines: Vec<PipelineConfig>,
    }

    impl TestConfig {
        pub(crate) fn new(pipeline_config: Option<PipelineConfig>) -> Self {
            Self {
                pipelines: vec![if let Some(pipeline_config) = pipeline_config {
                    pipeline_config
                } else {
                    PipelineConfig {
                        source: "test".to_string(),
                        sink: "test".to_string(),
                        translator: Some(TranslatorConfig {
                            from: "Uuid".to_string(),
                            to: "String".to_string(),
                        }),
                    }
                }],
                value: Value::Mapping(Mapping::new()),
            }
        }
    }

    impl Config for TestConfig {
        fn sink(&self, sink_identifier: &SinkIdentifier) -> Option<&Value> {
            info!("sink ID: {:?}", sink_identifier);
            Some(&self.value)
        }

        fn source(&self, source_identifier: &SourceIdentifier) -> Option<&Value> {
            info!("source ID: {:?}", source_identifier);
            Some(&self.value)
        }

        fn pipelines(&self) -> &Vec<PipelineConfig> {
            info!("pipelines");
            &self.pipelines
        }

        fn sink_configured(&self, name: &str) -> bool {
            name == "test"
        }

        fn source_configured(&self, name: &str) -> bool {
            name == "test"
        }
    }
}
