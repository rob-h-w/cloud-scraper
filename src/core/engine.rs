use std::sync::Arc;

use async_trait::async_trait;
use log::trace;
use tokio::task::{AbortHandle, JoinSet};
use tokio::time::sleep;

use crate::core::node_handles::NodeHandles;
use crate::domain::config::Config;
use crate::domain::mpsc_handle::{one_shot, OneshotMpscSenderHandle};
use crate::domain::node::{LifecycleChannelHandle, Manager};
use crate::integration::google::Source as GoogleSource;
use crate::integration::log::Sink as LogSink;
use crate::integration::stub::Source as StubSource;
use crate::server::WebServer;
use core::time::Duration;
#[cfg(test)]
use mockall::automock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use tokio::{join, task};

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
    stopped: Arc<AtomicBool>,
}

#[async_trait]
impl<ConfigType, ServerType> Engine for EngineImpl<ConfigType, ServerType>
where
    ConfigType: Config,
    ServerType: WebServer,
{
    async fn start(&self) {
        self.run().await;
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
            stopped: Arc::new(AtomicBool::new(false)),
        }
    }

    async fn run(&self) {
        trace!("run");
        let mut join_set = JoinSet::new();
        let mut abort_handles = Vec::new();

        let wait = self.config.exit_after();

        let mut stub_source = StubSource::new(&self.manager);
        let google_source = GoogleSource::new(&self.manager);
        let mut log_sink = LogSink::new(&self.manager, &stub_source.get_readonly_channel_handle());

        let node_handles = NodeHandles::new(&self.manager, google_source.control_events());

        abort_handles.push(join_set.spawn(async move { log_sink.run().await }));
        abort_handles.push(join_set.spawn(async move { stub_source.run().await }));
        abort_handles.push(join_set.spawn(async move { google_source.run().await }));

        let server = self.server.clone();
        abort_handles.push(join_set.spawn(async move {
            trace!("server starting");
            if let Err(e) = server.serve(&node_handles).await {
                log::error!("Error while serving: {}", e);
            }
        }));

        let (sender, join_set, abort_handle) = self.wait_for_init_responses(&mut join_set).await;
        abort_handles.push(abort_handle);
        let join_set = self.send_init(join_set, sender);

        let join_set = self.wait_for_timeout(join_set, wait).await;

        join!(Self::join(join_set), self.stop_checker(abort_handles));
        trace!("run completed");
    }

    async fn join(join_set: &mut JoinSet<()>) {
        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(_) => {}
                Err(e) => {
                    log::error!("Error while running task: {}", e);
                }
            }
        }
    }

    async fn wait_for_init_responses<'a>(
        &'a self,
        join_set: &'a mut JoinSet<()>,
    ) -> (OneshotMpscSenderHandle<()>, &mut JoinSet<()>, AbortHandle) {
        let readonly_manager = self.manager.readonly();
        let (sender, mut receiver) = one_shot();
        let expected = join_set.len();
        let waiting = Arc::new(AtomicBool::new(false));
        let moved_waiting = waiting.clone();
        let abort_handle = join_set.spawn(async move {
            trace!("init wait outer start");
            let task = task::spawn(async move {
                trace!("init wait inner start");
                moved_waiting.store(true, SeqCst);
                let mut count = 0;
                while count < expected {
                    receiver
                        .recv()
                        .await
                        .expect("Could not receive one shot init signal.");
                    trace!("Received init signal {}.", count);
                    count += 1;
                }
                trace!("init wait inner complete");
            });

            let stop_task = readonly_manager.abort_on_stop(&task);

            let _ = join!(task, stop_task);
            trace!("init wait outer complete");
        });

        while !waiting.load(SeqCst) {
            trace!("waiting init start");
            sleep(Duration::from_millis(0)).await;
        }

        (sender, join_set, abort_handle)
    }

    async fn wait_for_timeout<'a>(
        &'a self,
        join_set: &'a mut JoinSet<()>,
        wait: Option<Duration>,
    ) -> &mut JoinSet<()> {
        let mut wait_manager = self.manager.clone();
        let timer_started = Arc::new(AtomicBool::new(false));
        let moved_timer_started = timer_started.clone();
        let stopped = self.stopped.clone();

        join_set.spawn(async move {
            match wait {
                Some(duration) => {
                    trace!("timeout starting sleep");
                    moved_timer_started.store(true, SeqCst);
                    sleep(duration).await;
                    trace!("timeout sleep over");
                    match wait_manager.send_stop() {
                        Ok(_) => {
                            trace!("Lifecycle sent stop signal after {:?}", duration);
                            trace!("Lifecycle sent stop signal after {:?}", duration);
                        }
                        Err(e) => {
                            panic!("Lifecycle error while sending stop signal: {:?}", e);
                        }
                    }
                    stopped.store(true, SeqCst);
                }
                None => {
                    moved_timer_started.store(true, SeqCst);
                }
            }
        });

        while !timer_started.load(SeqCst) {
            trace!("waiting for timeout started start");
            sleep(Duration::from_millis(0)).await;
        }

        join_set
    }

    fn send_init<'a>(
        &'a self,
        join_set: &'a mut JoinSet<()>,
        sender: OneshotMpscSenderHandle<()>,
    ) -> &mut JoinSet<()> {
        let mut manager = self.manager.clone();
        manager
            .send_init(sender)
            .expect("Could not send init signal.");

        join_set
    }

    /// Stop all tasks when the stop signal is received.
    /// Used as a backup in case a task handles the stop signal incorrectly.
    async fn stop_checker(&self, abort_handles: Vec<AbortHandle>) {
        loop {
            if self.stopped.load(SeqCst) {
                trace!("Stopping all tasks.");
                abort_handles.iter().for_each(|handle| {
                    handle.abort();
                });
                break;
            }
            sleep(Duration::from_millis(10)).await;
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
        mock_web_server.expect_clone().times(1).returning(|| {
            let mut returned_mock_web_server = MockWebServer::new();
            returned_mock_web_server
                .expect_serve()
                .times(1)
                .returning(|_| Ok(()));
            returned_mock_web_server
        });

        block_on!(EngineImpl::new(Arc::new(StubConfig {}), mock_web_server).start());
    }
}
