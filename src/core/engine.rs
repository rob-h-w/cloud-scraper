use std::error::Error;
use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::config::Config;
use crate::static_init::pipelines::create_pipelines;
use crate::static_init::sinks::create_sinks;
use crate::static_init::sources::create_sources;
use crate::static_init::translators::create_translators;

pub(crate) struct EngineImpl<T>
where
    T: Config,
{
    config: Arc<T>,
}

#[async_trait]
pub(crate) trait Engine<T>
where
    T: Config,
{
    fn new(config: Arc<T>) -> Box<Self>;
    async fn start(&self) -> Result<(), Box<dyn Error>>;
}

#[async_trait]
impl<T> Engine<T> for EngineImpl<T>
where
    T: Config,
{
    fn new(config: Arc<T>) -> Box<EngineImpl<T>> {
        Box::new(EngineImpl { config })
    }

    async fn start(&self) -> Result<(), Box<dyn Error + 'static>> {
        let sources = create_sources(self.config.as_ref());
        let sinks = create_sinks(self.config.as_ref());
        let translators = create_translators();
        let _pipelines =
            create_pipelines(self.config.as_ref(), sources.as_ref(), &sinks, &translators);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use once_cell::sync::Lazy;
    use serde_yaml::Value;

    use crate::block_on;
    use crate::domain::config::{PipelineConfig, TranslatorConfig};
    use crate::domain::source_identifier::SourceIdentifier;

    use super::*;

    struct StubConfig {}

    impl Config for StubConfig {
        fn sink(&self, _sink_identifier: &str) -> Option<&Value> {
            todo!()
        }

        fn source(&self, _source_identifier: &SourceIdentifier) -> Option<&Value> {
            todo!()
        }

        fn pipelines(&self) -> &Vec<PipelineConfig> {
            static IT: Lazy<Vec<PipelineConfig>> = Lazy::new(|| {
                vec![PipelineConfig::new(
                    "log",
                    "stub",
                    Some(TranslatorConfig::new("uuid::Uuid", "alloc::string::String")),
                )]
            });
            &IT
        }

        fn sink_names(&self) -> Vec<String> {
            vec!["log".to_string()]
        }

        fn sink_configured(&self, name: &str) -> bool {
            name == "log"
        }

        fn source_configured(&self, name: &str) -> bool {
            name == "stub"
        }
    }

    #[test]
    fn test_engine_start() {
        block_on!(EngineImpl::new(Arc::new(StubConfig {})).start()).unwrap();
    }
}
