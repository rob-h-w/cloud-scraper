use std::sync::Arc;

use async_trait::async_trait;
use tokio::task::JoinSet;
use tokio::time::sleep;

use crate::domain::config::Config;
use crate::server::WebServer;

use crate::domain::channel_handle::ChannelHandle;
use crate::integration::log::Sink as LogSink;
use crate::integration::stub::Source as StubSource;
#[cfg(test)]
use mockall::automock;
use tokio::join;

#[macro_export]
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
    stop: ChannelHandle<bool>,
}

#[async_trait]
impl<ConfigType, ServerType> Engine for EngineImpl<ConfigType, ServerType>
where
    ConfigType: Config,
    ServerType: WebServer,
{
    async fn start(&self) {
        let (server_result, _) = join!(self.server.serve(self.stop.get_receiver()), self.run());

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
            server,
            stop: ChannelHandle::new(),
        }
    }

    async fn run(&self) {
        let mut wait_channel_handle = self.stop.clone();
        let mut join_set = JoinSet::new();

        let wait = self.config.exit_after();
        let _timer = join_set.spawn(async move {
            match wait {
                Some(duration) => {
                    sleep(duration).await;
                    match wait_channel_handle.send(true) {
                        Ok(_) => {
                            log::trace!("Wait sent stop signal after {:?}", duration);
                        }
                        Err(e) => {
                            panic!("Wait error while sending stop signal: {:?}", e);
                        }
                    }
                }
                None => {}
            }
        });

        let mut stub_source = StubSource::new(self.stop.read_only());
        let mut log_sink = LogSink::new(&self.stop, &stub_source.get_readonly_channel_handle());

        join_set.spawn(async move { log_sink.run().await });
        join_set.spawn(async move { stub_source.run().await });

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
