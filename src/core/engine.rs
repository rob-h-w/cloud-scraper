use std::sync::Arc;

use async_trait::async_trait;
use tokio::task::JoinSet;
use tokio::time::sleep;

use crate::domain::config::Config;
use crate::server::WebServer;

use crate::domain::node::{LifecycleChannelHandle, Manager};
use crate::integration::google::keep::source::Source as GoogleSource;
use crate::integration::log::Sink as LogSink;
use crate::integration::stub::Source as StubSource;
#[cfg(test)]
use mockall::automock;
use tokio::join;

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
    manager: Manager,
    server: ServerType,
}

#[async_trait]
impl<ConfigType, ServerType> Engine for EngineImpl<ConfigType, ServerType>
where
    ConfigType: Config,
    ServerType: WebServer,
{
    async fn start(&self) {
        let (server_result, _) = join!(
            self.server.serve(self.manager.readonly().get_receiver()),
            self.run()
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
        Self {
            config,
            manager: Manager::new(LifecycleChannelHandle::new()),
            server,
        }
    }

    async fn run(&self) {
        let mut join_set = JoinSet::new();

        let wait = self.config.exit_after();
        let mut wait_manager = self.manager.clone();
        let _timer = join_set.spawn(async move {
            match wait {
                Some(duration) => {
                    sleep(duration).await;
                    match wait_manager.stop() {
                        Ok(_) => {
                            log::trace!("Lifecycle sent stop signal after {:?}", duration);
                        }
                        Err(e) => {
                            panic!("Lifecycle error while sending stop signal: {:?}", e);
                        }
                    }
                }
                None => {}
            }
        });

        let mut stub_source = StubSource::new(&self.manager);
        let google_source = GoogleSource::new(&self.manager);
        let mut log_sink = LogSink::new(&self.manager, &stub_source.get_readonly_channel_handle());

        join_set.spawn(async move { log_sink.run().await });
        join_set.spawn(async move { stub_source.run().await });
        join_set.spawn(async move { google_source.run().await });

        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(_) => {}
                Err(e) => {
                    log::error!("Error while running task: {}", e);
                }
            }
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
