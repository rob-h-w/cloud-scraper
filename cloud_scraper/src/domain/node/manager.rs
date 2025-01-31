use crate::domain::config::Config;
use crate::domain::mpsc_handle::OneshotMpscSenderHandle;
use crate::domain::node::InitReplier;
use crate::domain::node::Lifecycle::{Init, ReadConfig, Stop};
use crate::domain::node::{Lifecycle, LifecycleChannelHandle};
use log::{debug, error};
use std::any::TypeId;
use std::sync::Arc;
use tokio::sync::broadcast::error::{RecvError, SendError};
use tokio::sync::broadcast::Receiver;
use tokio::sync::Semaphore;
use tokio::task;
use tokio::task::JoinHandle;

#[derive(Clone, Debug)]
pub struct Manager {
    config: Arc<Config>,
    lifecycle_channel_handle: LifecycleChannelHandle,
}

impl Manager {
    pub fn new(config: &Arc<Config>, lifecycle_channel_handle: LifecycleChannelHandle) -> Self {
        Self {
            config: config.clone(),
            lifecycle_channel_handle,
        }
    }

    pub fn core_config(&self) -> &Config {
        self.config.as_ref()
    }

    pub fn readonly(&self) -> ReadonlyManager {
        ReadonlyManager::new(self)
    }

    pub fn send_init(
        &mut self,
        sender: OneshotMpscSenderHandle<()>,
    ) -> Result<(), SendError<Lifecycle>> {
        self.lifecycle_channel_handle.send(Init(sender))?;
        Ok(())
    }

    pub fn send_read_config<T: 'static>(&mut self) -> Result<usize, SendError<Lifecycle>> {
        self.lifecycle_channel_handle
            .send(ReadConfig(TypeId::of::<T>()))
    }

    pub fn send_stop(&mut self) -> Result<usize, SendError<Lifecycle>> {
        self.lifecycle_channel_handle.send(Stop)
    }
}

#[derive(Clone, Debug)]
pub struct ReadonlyManager {
    manager: Manager,
}

impl ReadonlyManager {
    fn new(manager: &Manager) -> Self {
        Self {
            manager: manager.clone(),
        }
    }

    pub(crate) async fn abort_on_stop<T>(&self, task: &JoinHandle<T>) -> JoinHandle<()> {
        abort_on_stop::<T>(self.manager.lifecycle_channel_handle.get_receiver(), task).await
    }

    pub(crate) fn get_receiver(&self) -> Receiver<Lifecycle> {
        self.manager.lifecycle_channel_handle.get_receiver()
    }
}

pub(crate) async fn abort_on_stop<T>(
    mut event_receiver: Receiver<Lifecycle>,
    task: &JoinHandle<T>,
) -> JoinHandle<()> {
    let task_abort_handle = task.abort_handle();
    let semaphore = Arc::new(Semaphore::new(1));
    let permit = semaphore
        .clone()
        .acquire_owned()
        .await
        .expect("Could not acquire semaphore");
    let handle = task::spawn(async move {
        drop(permit);
        loop {
            match event_receiver.recv().await {
                Ok(event) => match event {
                    Init(event) => event.reply_to_init_with((), "abort_on_stop").await,
                    Stop => {
                        task_abort_handle.abort();
                        break;
                    }
                    _ => {}
                },
                Err(e) => match e {
                    RecvError::Closed => {
                        debug!("Channel closed in abort_on_stop.");
                        break;
                    }
                    RecvError::Lagged(amount) => {
                        error!("Lagged amount of {} in abort_on_stop.", amount);
                    }
                },
            }
        }
    });

    let _permit = semaphore
        .acquire()
        .await
        .expect("Could not acquire semaphore");
    handle
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn get_test_manager(config: &Arc<Config>) -> Manager {
        let lifecycle_handle = LifecycleChannelHandle::new();
        Manager::new(config, lifecycle_handle)
    }

    mod send_read_config {
        use super::*;
        use crate::domain::config::tests::test_config;
        use crate::domain::node::manager::tests::get_test_manager;
        use crate::domain::node::tests::TestNode;
        use tokio_test::assert_ok;

        #[tokio::test]
        async fn test_send_read_config() {
            let config = test_config();
            let mut manager = get_test_manager(&config);
            let mut lifecycle_receiver = manager.readonly().get_receiver();
            let result = manager.send_read_config::<TestNode>();
            assert_ok!(result);

            let event = lifecycle_receiver.recv().await.unwrap();
            assert_eq!(event, ReadConfig(TypeId::of::<TestNode>()));
        }
    }
}
