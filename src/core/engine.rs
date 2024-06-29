use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::task::{AbortHandle, JoinSet};
use tokio::time::{sleep, Duration};

use crate::core::error::PipelineError;
use crate::core::pipeline::ExecutablePipeline;
use crate::domain::config::Config;
use crate::server::WebServer;
use crate::static_init::pipelines::create_pipelines;
use crate::static_init::sinks::create_sinks;
use crate::static_init::sources::create_sources;
use crate::static_init::translators::create_translators;

#[cfg(test)]
use mockall::automock;
use tokio::join;

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

#[async_trait]
#[cfg_attr(test, automock)]
pub(crate) trait Engine {
    async fn start(&self);
}

pub(crate) struct EngineImpl<ConfigType, ServerType>
where
    ConfigType: Config,
    ServerType: WebServer,
{
    config: Arc<ConfigType>,
    server: ServerType,
    stop: tokio::sync::broadcast::Sender<bool>,
}

#[async_trait]
impl<ConfigType, ServerType> Engine for EngineImpl<ConfigType, ServerType>
where
    ConfigType: Config,
    ServerType: WebServer,
{
    async fn start(&self) {
        let (server_result, _) = join!(
            self.server.serve(self.stop.subscribe()),
            self.run_pipelines()
        );

        if let Err(e) = server_result {
            log::error!("Error while serving: {}", e);
        }
    }
}

impl<ConfigType, ServerType> EngineImpl<ConfigType, ServerType>
where
    ConfigType: Config,
    ServerType: WebServer,
{
    pub(crate) fn new(config: Arc<ConfigType>, server: ServerType) -> Self {
        let (stop_tx, _) = tokio::sync::broadcast::channel(32);
        Self {
            config,
            server,
            stop: stop_tx,
        }
    }

    async fn run_pipelines(&self) {
        let sources = create_sources(self.config.as_ref());
        let sinks = create_sinks(self.config.as_ref());
        let translators = create_translators();
        let mut pipelines =
            create_pipelines(self.config.as_ref(), sources.as_ref(), &sinks, &translators);

        if !pipelines.is_empty() {
            let (count_tx, mut count_rx) = tokio::sync::mpsc::channel(pipelines.len());
            let (error_tx, mut error_rx) = tokio::sync::mpsc::channel(pipelines.len());

            let mut counter_stop_rx = self.stop.subscribe();
            let wait_stop_tx = self.stop.clone();

            let mut join_set = JoinSet::new();
            let mut cancellation_handles: Vec<AbortHandle> = vec![];
            while let Some(pipeline) = pipelines.pop() {
                let count_tx = count_tx.clone();
                let error_tx = error_tx.clone();
                let stop_rx = self.stop.subscribe();
                cancellation_handles.push(join_set.spawn(async move {
                    run_pipeline(pipeline, count_tx, error_tx, stop_rx).await
                }));
            }

            let wait = self.config.exit_after();
            let mut wait_stop_rx = self.stop.subscribe();
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

            let mut error_stop = self.stop.subscribe();
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

            let result = counter.await.unwrap_or_else(|e| {
                log::error!("Error while waiting for counter: {}", e);
                0
            });

            log::info!("Processed {} entities", result);
        } else {
            match self.stop.send(true) {
                Ok(_) => {
                    log::trace!("No pipelines to run, sent stop signal");
                }
                Err(e) => {
                    log::error!(
                        "No pipelines to run, error while sending stop signal: {}",
                        e
                    );
                }
            }
        }
    }
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
pub(crate) mod tests {
    use once_cell::sync::Lazy;
    use serde_yaml::Value;
    use std::time::Duration;

    use crate::block_on;
    use crate::domain::config::{DomainConfig, PipelineConfig, TranslatorConfig};
    use crate::domain::source_identifier::SourceIdentifier;
    use crate::server::MockWebServer;

    use super::*;

    pub(crate) struct StubConfig {}

    impl Config for StubConfig {
        fn domain_config(&self) -> Option<&DomainConfig> {
            None
        }

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

        fn port(&self) -> u16 {
            80
        }

        fn sink_names(&self) -> Vec<String> {
            vec!["log".to_string()]
        }

        fn sink_configured(&self, name: &str) -> bool {
            name == "log"
        }

        fn site_folder(&self) -> &str {
            "./stub_site_folder"
        }

        fn source_configured(&self, name: &str) -> bool {
            name == "stub"
        }
    }

    #[test]
    fn test_engine_start() {
        let mut mock_web_server = MockWebServer::new();
        mock_web_server
            .expect_serve()
            .times(1)
            .returning(|_| Box::pin(async { Ok(()) }));
        block_on!(EngineImpl::new(Arc::new(StubConfig {}), mock_web_server).start());
    }
}
