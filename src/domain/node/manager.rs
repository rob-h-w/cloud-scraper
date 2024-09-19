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
use tokio::task;
use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct Manager {
    config: Arc<Config>,
    lifecycle_channel_handle: LifecycleChannelHandle,
}

impl Clone for Manager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            lifecycle_channel_handle: self.lifecycle_channel_handle.clone(),
        }
    }
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

    pub fn abort_on_stop<T>(&self, task: &JoinHandle<T>) -> JoinHandle<()> {
        abort_on_stop::<T>(self.manager.lifecycle_channel_handle.get_receiver(), task)
    }

    pub fn get_receiver(&self) -> Receiver<Lifecycle> {
        self.manager.lifecycle_channel_handle.get_receiver()
    }
}

pub fn abort_on_stop<T>(
    mut event_receiver: Receiver<Lifecycle>,
    task: &JoinHandle<T>,
) -> JoinHandle<()> {
    let task_abort_handle = task.abort_handle();
    task::spawn(async move {
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
    })
}

#[cfg(test)]
pub mod test {
    use super::*;

    pub fn get_test_manager(config: &Arc<Config>) -> Manager {
        let lifecycle_handle = LifecycleChannelHandle::new();
        Manager::new(config, lifecycle_handle)
    }

    mod send_read_config {
        use super::*;
        use crate::domain::config::tests::test_config;
        use crate::domain::node::manager::test::get_test_manager;
        use crate::domain::node::test::TestNode;
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
