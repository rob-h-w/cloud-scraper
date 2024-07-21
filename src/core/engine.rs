use std::sync::Arc;

use async_trait::async_trait;
use tokio::task::JoinSet;
use tokio::time::sleep;

use crate::domain::config::Config;
use crate::server::WebServer;
use crate::static_init::sinks::create_sinks;
use crate::static_init::sources::create_sources;

#[cfg(test)]
use mockall::automock;
use tokio::join;

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
        let (server_result, _) = join!(self.server.serve(self.stop.subscribe()), self.run());

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

    async fn run(&self) {
        let _sources = create_sources(self.config.as_ref());
        let _sinks = create_sinks(self.config.as_ref());

        let (_error_tx, mut error_rx) = tokio::sync::mpsc::channel::<bool>(16);
        let wait_stop_tx = self.stop.clone();
        let mut wait_stop_rx = self.stop.subscribe();
        let mut join_set = JoinSet::new();

        let wait = self.config.exit_after();
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
        let mut cancelled = false;
        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(_) => {}
                Err(e) => {
                    log::error!("Error while running pipeline: {}", e);
                    if !cancelled {
                        cancelled = true;
                        timer.abort();
                        error.abort();
                    }
                }
            }
        }

        if !error.is_finished() {
            error.abort();
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::time::Duration;

    use crate::block_on;
    use crate::domain::config::DomainConfig;
    use crate::server::MockWebServer;

    use super::*;

    pub(crate) struct StubConfig {}

    impl Config for StubConfig {
        fn domain_config(&self) -> Option<&DomainConfig> {
            None
        }

        fn email(&self) -> Option<&str> {
            None
        }

        fn exit_after(&self) -> Option<Duration> {
            Some(Duration::from_millis(10))
        }

        fn port(&self) -> u16 {
            80
        }

        fn site_folder(&self) -> &str {
            "./stub_site_folder"
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
