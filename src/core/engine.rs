use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::task::{AbortHandle, JoinSet};
use tokio::time::{sleep, Duration};

use crate::core::error::PipelineError;
use crate::core::pipeline::ExecutablePipeline;
use crate::domain::config::Config;
use crate::static_init::pipelines::create_pipelines;
use crate::static_init::sinks::create_sinks;
use crate::static_init::sources::create_sources;
use crate::static_init::translators::create_translators;

const POLL_COOLOFF: Duration = Duration::from_millis(100);

macro_rules! do_until_stop {
    ($stop_rx:expr, $f:expr) => {
        loop {
            if !$stop_rx.is_empty() {
                match $stop_rx.try_recv() {
                    Ok(stop) => {
                        if stop {
                            log::trace!("Received stop signal");
                            break;
                        }
                    }
                    Err(e) => {
                        log::error!("Error while receiving stop signal: {}", e);
                    }
                }
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

    async fn start(&self) {
        let sources = create_sources(self.config.as_ref());
        let sinks = create_sinks(self.config.as_ref());
        let translators = create_translators();
        let mut pipelines =
            create_pipelines(self.config.as_ref(), sources.as_ref(), &sinks, &translators);
        let result = run_pipelines(self.config.as_ref(), &mut pipelines).await;

        log::info!("Processed {} entities", result);
    }
}

async fn run_pipelines<T: Config>(
    config: &T,
    pipelines: &mut Vec<Box<dyn ExecutablePipeline>>,
) -> usize {
    if pipelines.is_empty() {
        return 0;
    }

    let (count_tx, mut count_rx) = tokio::sync::mpsc::channel(pipelines.len());
    let (error_tx, mut error_rx) = tokio::sync::mpsc::channel(pipelines.len());
    let (stop_tx, _) = tokio::sync::broadcast::channel(pipelines.len());

    let mut counter_stop_rx = stop_tx.subscribe();
    let wait_stop_tx = stop_tx.clone();

    let mut join_set = JoinSet::new();
    let mut cancellation_handles: Vec<AbortHandle> = vec![];
    while let Some(pipeline) = pipelines.pop() {
        let count_tx = count_tx.clone();
        let error_tx = error_tx.clone();
        let stop_rx = stop_tx.subscribe();
        cancellation_handles.push(
            join_set
                .spawn(async move { run_pipeline(pipeline, count_tx, error_tx, stop_rx).await }),
        );
    }

    let wait = config.exit_after();
    let mut wait_stop_rx = stop_tx.subscribe();
    let timer = join_set.spawn(async move {
        match wait {
            Some(duration) => {
                sleep(duration).await;
                match wait_stop_tx.send(true) {
                    Ok(_) => {
                        log::trace!("Wait sent stop signal after {:?}", duration);
                    }
                    Err(e) => {
                        panic!("Wait error while sending stop signal: {}", e);
                    }
                }
            }
            None => match wait_stop_rx.recv().await {
                Ok(stop) => {
                    log::trace!("Wait received stop signal: {}", stop);
                }
                Err(e) => {
                    log::error!("Wait error while waiting for stop signal: {}", e);
                }
            },
        }
    });

    let mut error_stop = stop_tx.subscribe();
    let error = tokio::spawn(async move {
        do_until_stop!(error_stop, {
            match error_rx.recv().await {
                Some(e) => log::error!("Error while running pipeline: {}", e),
                None => break,
            }
        });
    });

    let counter = tokio::spawn(async move {
        let mut count = 0;
        do_until_stop!(counter_stop_rx, {
            log::trace!("Waiting for count");
            match count_rx.recv().await {
                Some(c) => {
                    log::trace!("Received count: {}", c);
                    count += c;
                }
                None => {
                    log::trace!("Count channel closed");
                    break;
                }
            }
            log::trace!("Count: {}, waiting for stop", count);
        });
        log::trace!("Count: {}", count);
        count
    });

    let mut cancelled = false;
    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(_) => {}
            Err(e) => {
                log::error!("Error while running pipeline: {}", e);
                if !cancelled {
                    cancelled = true;
                    cancellation_handles.iter().for_each(|h| h.abort());
                    timer.abort();
                    counter.abort();
                    error.abort();
                }
            }
        }
    }

    while !counter.is_finished() {
        // nudge the counter to finish by sending a 0.
        match count_tx.send(0).await {
            Ok(_) => {
                log::trace!("Sent count 0");
            }
            Err(e) => {
                log::trace!("Error while sending count: {}", e);

                // The nudge didn't work. Lose the count & abort.
                counter.abort();
            }
        }
    }

    if !error.is_finished() {
        error.abort();
    }

    counter.await.unwrap_or_else(|e| {
        log::error!("Error while waiting for counter: {}", e);
        0
    })
}

async fn run_pipeline(
    pipeline: Box<dyn ExecutablePipeline>,
    count_tx: Sender<usize>,
    error_tx: Sender<PipelineError>,
    mut stop_rx: Receiver<bool>,
) {
    let mut since = None;
    do_until_stop!(stop_rx, {
        match pipeline.run(since).await {
            Ok(count) => match count_tx.send(count).await {
                Ok(_) => {
                    log::trace!("Pipeline sent count: {}", count);
                }
                Err(e) => {
                    log::error!("Pipeline error while sending count: {}", e);
                }
            },
            Err(e) => match error_tx.send(e.clone()).await {
                Ok(_) => {
                    log::trace!("Pipeline sent error: {}", e);
                }
                Err(e) => {
                    log::error!("Pipeline error while sending error: {}", e);
                }
            },
        };

        since = Some(chrono::Utc::now());
        sleep(POLL_COOLOFF).await;
    })
}

#[cfg(test)]
mod tests {
    use once_cell::sync::Lazy;
    use serde_yaml::Value;
    use std::time::Duration;

    use crate::block_on;
    use crate::domain::config::{PipelineConfig, TranslatorConfig};
    use crate::domain::source_identifier::SourceIdentifier;

    use super::*;

    struct StubConfig {}

    impl Config for StubConfig {
        fn exit_after(&self) -> Option<Duration> {
            Some(Duration::from_millis(10))
        }
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
    async fn start(&self);
}
