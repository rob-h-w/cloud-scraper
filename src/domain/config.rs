use std::time::Duration;
use std::vec;

#[cfg(test)]
use crate::domain::source_identifier::SourceIdentifier;
use serde::Deserialize;
#[cfg(test)]
use serde_yaml::Value;

pub(crate) trait Config: Send + Sync {
    fn domain_config(&self) -> Option<&DomainConfig>;
    fn domain_is_defined(&self) -> bool {
        self.domain_config().is_some()
    }
    fn email(&self) -> Option<&str>;
    fn exit_after(&self) -> Option<Duration> {
        None
    }
    #[cfg(test)]
    fn sink(&self, sink_identifier: &str) -> Option<&Value>;
    #[cfg(test)]
    fn source(&self, source_identifier: &SourceIdentifier) -> Option<&Value>;
    fn pipelines(&self) -> &Vec<PipelineConfig>;
    fn port(&self) -> u16;

    fn sink_names(&self) -> Vec<String>;

    fn sink_configured(&self, name: &str) -> bool;
    fn site_folder(&self) -> &str;
    fn source_configured(&self, name: &str) -> bool;

    fn sanity_check(&self) -> Result<(), String> {
        let mut errors = vec![];

        if self.domain_is_defined() {
            if self.email().is_none() {
                errors.push("No email configured".to_string());
            }
        }

        for pipeline in self.pipelines() {
            if !self.sink_configured(&pipeline.sink) {
                errors.push(format!("Sink '{}' not configured", pipeline.sink));
            }
            if !self.source_configured(&pipeline.source) {
                errors.push(format!("Source '{}' not found", pipeline.source));
            }
        }

        if let Some(domain_config) = self.domain_config() {
            if domain_config.builder_contacts.is_empty() {
                errors.push("No builder contacts configured".to_string());
            }
            if domain_config.domain_name.is_empty() {
                errors.push("No domain name configured".to_string());
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join(", "))
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct DomainConfig {
    pub builder_contacts: Vec<String>,
    pub domain_name: String,
    pub poll_attempts: usize,
    pub poll_interval_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PipelineConfig {
    sink: String,
    source: String,
    translator: Option<TranslatorConfig>,
}

impl PipelineConfig {
    pub(crate) fn new(sink: &str, source: &str, translator: Option<TranslatorConfig>) -> Self {
        Self {
            sink: sink.to_string(),
            source: source.to_string(),
            translator,
        }
    }
    pub(crate) fn source(&self) -> &str {
        &self.source
    }

    pub(crate) fn sink(&self) -> &str {
        &self.sink
    }

    pub(crate) fn translator(&self) -> Option<&TranslatorConfig> {
        self.translator.as_ref()
    }
}

impl Clone for PipelineConfig {
    fn clone(&self) -> Self {
        PipelineConfig {
            sink: self.sink.clone(),
            source: self.source.clone(),
            translator: self.translator.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct TranslatorConfig {
    from: String,
    to: String,
}

impl TranslatorConfig {
    #[cfg(test)]
    pub(crate) fn new(from: &str, to: &str) -> Self {
        Self {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    pub(crate) fn from(&self) -> &str {
        &self.from
    }

    pub(crate) fn to(&self) -> &str {
        &self.to
    }
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
            let sink_config = config.sink(TestSink::SINK_ID).unwrap();
            assert_eq!(sink_config, &Value::Mapping(Mapping::new()));

            let log_entries = logger.log_entries();
            assert_eq!(log_entries.len(), 1);
            let log_line = &log_entries[0];
            assert_eq!(log_line.level(), log::Level::Info);
            assert!(log_line.args().contains("\"test\""));
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
        assert_eq!(translator.from, "uuid::Uuid");
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

        let domain_config = Some(DomainConfig {
            builder_contacts: vec!["builder@contact.com".to_string()],
            domain_name: "the.domain".to_string(),
            poll_attempts: 0,
            poll_interval_seconds: 0,
        });

        let bad_config = TestConfig::new_domain_email(domain_config.clone(), None);
        assert!(bad_config.sanity_check().is_err());

        let config = TestConfig::new_domain_email(domain_config, Some("the@email.com".to_string()));
        assert!(config.sanity_check().is_ok());
    }

    pub(crate) struct TestConfig {
        domain_config: Option<DomainConfig>,
        email: Option<String>,
        value: Value,
        pipelines: Vec<PipelineConfig>,
    }

    impl TestConfig {
        pub(crate) fn new(pipeline_config: Option<PipelineConfig>) -> Self {
            Self {
                domain_config: None,
                email: None,
                pipelines: vec![if let Some(pipeline_config) = pipeline_config {
                    pipeline_config
                } else {
                    PipelineConfig {
                        source: "test".to_string(),
                        sink: "test".to_string(),
                        translator: Some(TranslatorConfig::new("uuid::Uuid", "String")),
                    }
                }],
                value: Value::Mapping(Mapping::new()),
            }
        }

        pub fn new_domain_email(
            domain_config: Option<DomainConfig>,
            email: Option<String>,
        ) -> Self {
            Self {
                domain_config,
                email,
                pipelines: vec![PipelineConfig::new("test", "test", None)],
                value: Value::Mapping(Mapping::new()),
            }
        }
    }

    impl Config for TestConfig {
        fn domain_config(&self) -> Option<&DomainConfig> {
            self.domain_config.as_ref()
        }

        fn email(&self) -> Option<&str> {
            self.email.as_deref()
        }

        fn sink(&self, sink_identifier: &str) -> Option<&Value> {
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

        fn port(&self) -> u16 {
            80
        }

        fn sink_names(&self) -> Vec<String> {
            vec!["test".to_string()]
        }

        fn sink_configured(&self, name: &str) -> bool {
            name == "test"
        }

        fn site_folder(&self) -> &str {
            "test_site_folder"
        }

        fn source_configured(&self, name: &str) -> bool {
            name == "test"
        }
    }
}
