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
use tokio::sync::Semaphore;
use tokio::{join, task};

#[async_trait]
#[cfg_attr(test, automock)]
pub(crate) trait Engine {
    async fn start(&self);
}

pub(crate) struct EngineImpl<ServerType>
where
    ServerType: WebServer,
{
    manager: Manager,
    server: ServerType,
    stopped: Arc<AtomicBool>,
}

#[async_trait]
impl<ServerType> Engine for EngineImpl<ServerType>
where
    ServerType: WebServer,
{
    async fn start(&self) {
        self.run().await;
    }
}

impl<ServerType> EngineImpl<ServerType>
where
    ServerType: WebServer,
{
    pub(crate) fn new(config: &Arc<Config>, server: ServerType) -> Self {
        Self {
            manager: Manager::new(config, LifecycleChannelHandle::new()),
            server,
            stopped: Arc::new(AtomicBool::new(false)),
        }
    }

    async fn run(&self) {
        trace!("run");
        let mut join_set = JoinSet::new();
        let mut abort_handles = Vec::new();

        let wait_duration = self.manager.core_config().exit_after();

        let mut stub_source = StubSource::new(&self.manager);
        let google_source = GoogleSource::new(
            &self.manager,
            &self.server.get_flow_delegate_factory(&self.manager),
        );
        let mut log_sink = LogSink::new(&self.manager, &stub_source.get_readonly_channel_handle());

        let node_handles = NodeHandles::new(
            &self.manager,
            google_source.control_events(),
            &self.server.get_web_channel_handle(),
        );

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

        let join_set = self.wait_for_timeout(join_set, wait_duration).await;

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
        let semaphore = Arc::new(Semaphore::new(1));
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("Could not acquire semaphore");
        let abort_handle = join_set.spawn(async move {
            trace!("init wait outer start");
            let task = task::spawn(async move {
                trace!("init wait inner start");
                drop(permit);
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

        let _permit = semaphore
            .acquire()
            .await
            .expect("Could not acquire semaphore");

        (sender, join_set, abort_handle)
    }

    async fn wait_for_timeout<'a>(
        &'a self,
        join_set: &'a mut JoinSet<()>,
        wait: Option<Duration>,
    ) -> &mut JoinSet<()> {
        let mut wait_manager = self.manager.clone();
        let semaphore = Arc::new(Semaphore::new(1));
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("Could not acquire semaphore");
        let stopped = self.stopped.clone();

        join_set.spawn(async move {
            drop(permit);
            match wait {
                Some(duration) => {
                    trace!("timeout starting sleep");
                    sleep(duration).await;
                    trace!("timeout sleep over");
                    match wait_manager.send_stop() {
                        Ok(_) => {
                            trace!("Lifecycle sent stop signal after {:?}", duration);
                        }
                        Err(e) => {
                            panic!("Lifecycle error while sending stop signal: {:?}", e);
                        }
                    }
                    stopped.store(true, SeqCst);
                }
                None => {}
            }
        });

        let _permit = semaphore
            .acquire()
            .await
            .expect("Could not acquire semaphore");

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
    use crate::block_on;
    use crate::server::{MockWebServer, OauthFlowDelegateFactory, WebEventChannelHandle};

    use super::*;

    pub fn stub_config() -> Arc<Config> {
        Arc::new(Config::with_all_properties(
            None,
            None,
            Some(1),
            Some(80),
            Some("./stub_site_folder".to_string()),
        ))
    }

    #[test]
    fn test_engine_start() {
        let web_channel_handle = WebEventChannelHandle::new();
        let cloned_web_channel_handle = web_channel_handle.clone();
        let mut mock_web_server = MockWebServer::new();
        mock_web_server.expect_clone().times(1).returning(|| {
            let mut returned_mock_web_server = MockWebServer::new();
            returned_mock_web_server
                .expect_serve()
                .times(1)
                .returning(|_| Ok(()));
            returned_mock_web_server
        });
        mock_web_server
            .expect_get_flow_delegate_factory()
            .returning(move |manager| OauthFlowDelegateFactory::new(manager, &web_channel_handle));
        mock_web_server
            .expect_get_web_channel_handle()
            .return_const(cloned_web_channel_handle.clone());

        block_on!(EngineImpl::new(&stub_config(), mock_web_server).start());
    }
}
