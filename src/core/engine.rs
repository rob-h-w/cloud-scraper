use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc::Sender;
use tokio::sync::watch::Receiver;
use tokio::task::JoinHandle;

use crate::core::error::PipelineError;
use crate::core::pipeline::ExecutablePipeline;
use crate::domain::config::Config;
use crate::static_init::pipelines::create_pipelines;
use crate::static_init::sinks::create_sinks;
use crate::static_init::sources::create_sources;
use crate::static_init::translators::create_translators;

macro_rules! do_until_stop {
    ($stop_rx:expr, $f:expr) => {
        loop {
            if $stop_rx.has_changed().unwrap() {
                break;
            }
            $f;
        }
    };
}

pub(crate) struct EngineImpl<T>
where
    T: Config,
{
    config: Arc<T>,
}

#[async_trait]
impl<T> Engine<T> for EngineImpl<T>
where
    T: Config,
{
    fn new(config: Arc<T>) -> Box<EngineImpl<T>> {
        Box::new(EngineImpl { config })
    }

    async fn start(&self) -> Vec<String> {
        let sources = create_sources(self.config.as_ref());
        let sinks = create_sinks(self.config.as_ref());
        let translators = create_translators();
        let mut pipelines =
            create_pipelines(self.config.as_ref(), sources.as_ref(), &sinks, &translators);
        let result = run_pipelines(&mut pipelines).await;

        match result {
            Ok(count) => {
                format!("Processed {} entities", count);
                vec![]
            }
            Err(errors) => errors
                .iter()
                .filter_map(|e| {
                    log::error!("Error while running pipeline: {}", e);
                    Some(e.to_string())
                })
                .collect(),
        }
    }
}

async fn run_pipelines(
    pipelines: &mut Vec<Box<dyn ExecutablePipeline>>,
) -> Result<usize, Vec<PipelineError>> {
    let (count_tx, mut count_rx) = tokio::sync::mpsc::channel(pipelines.len());
    let (error_tx, mut error_rx) = tokio::sync::mpsc::channel(pipelines.len());
    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);

    let mut futures: Vec<JoinHandle<()>> = vec![];

    while !pipelines.is_empty() {
        let pipeline = pipelines.pop().unwrap();
        let count_tx = count_tx.clone();
        let error_tx = error_tx.clone();
        let stop_rx = stop_rx.clone();
        futures.push(tokio::spawn(async move {
            run_pipeline(pipeline, count_tx, error_tx, stop_rx).await;
        }));
    }

    let error_watcher = tokio::spawn(async move {
        error_rx.recv().await;
        stop_tx.send(true).unwrap();
    });

    let counter = tokio::spawn(async move {
        let mut count = 0;
        do_until_stop!(stop_rx.clone(), {
            count += count_rx.recv().await.unwrap_or(0);
        });
        count
    });

    Ok(0)
}

async fn run_pipeline(
    pipeline: Box<dyn ExecutablePipeline>,
    count_tx: Sender<usize>,
    error_tx: Sender<PipelineError>,
    mut stop_rx: Receiver<bool>,
) {
    let mut since = None;
    do_until_stop!(stop_rx.clone(), {
        match pipeline.run(since).await {
            Ok(count) => count_tx.send(count).await.unwrap(),
            Err(e) => error_tx.send(e).await.unwrap(),
        }

        since = Some(chrono::Utc::now());
    })
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
        block_on!(EngineImpl::new(Arc::new(StubConfig {})).start());
    }
}

#[async_trait]
pub(crate) trait Engine<T>
where
    T: Config,
{
    fn new(config: Arc<T>) -> Box<Self>;
    async fn start(&self) -> Vec<String>;
}
