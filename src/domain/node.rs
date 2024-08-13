use crate::domain::channel_handle::ChannelHandle;
use crate::domain::config::Config;
use crate::domain::mpsc_handle::OneshotMpscSenderHandle;
use crate::domain::node::Lifecycle::{Init, ReadConfig, Redirect};
use async_trait::async_trait;
use log::error;
use std::any::TypeId;
use std::sync::Arc;
use tokio::sync::broadcast::error::SendError;
use tokio::sync::broadcast::Receiver;
use tokio::task;
use tokio::task::JoinHandle;
use Lifecycle::Stop;

#[derive(Clone, Debug, PartialEq)]
pub enum Lifecycle {
    Init(OneshotMpscSenderHandle<()>),
    ReadConfig(TypeId),
    Redirect(String, OneshotMpscSenderHandle<String>),
    Stop,
}

pub trait LifecycleAware {
    fn is_stop(&self) -> bool;
    fn is_this<T>(&self) -> bool
    where
        T: 'static;
}

impl LifecycleAware for Lifecycle {
    fn is_stop(&self) -> bool {
        self == &Stop
    }

    fn is_this<T: 'static>(&self) -> bool {
        match self {
            ReadConfig(type_id) => type_id == &TypeId::of::<T>(),
            _ => false,
        }
    }
}

impl LifecycleAware for Option<Lifecycle> {
    fn is_stop(&self) -> bool {
        if let Some(it) = self {
            it.is_stop()
        } else {
            false
        }
    }

    fn is_this<T>(&self) -> bool
    where
        T: 'static,
    {
        if let Some(it) = self {
            it.is_this::<T>()
        } else {
            false
        }
    }
}

impl<E: std::fmt::Debug> LifecycleAware for Result<Lifecycle, E> {
    fn is_stop(&self) -> bool {
        match self {
            Ok(lifecycle) => lifecycle.is_stop(),
            Err(e) => {
                panic!("Could not get if stop lifecycle event because of {:?}", e)
            }
        }
    }

    fn is_this<T>(&self) -> bool
    where
        T: 'static,
    {
        match self {
            Ok(lifecycle) => lifecycle.is_this::<T>(),
            Err(e) => {
                panic!("Could not get if init lifecycle event because of {:?}", e)
            }
        }
    }
}

#[async_trait]
pub trait InitReplier<T> {
    async fn reply_to_init_with(&self, value: T, sent_in: &str);
    async fn send(&self, value: T) -> Result<(), ()>;
}

pub type LifecycleChannelHandle = ChannelHandle<Lifecycle>;

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

    pub fn send_oauth2_redirect_url(
        &mut self,
        url: &str,
        _need_code: bool,
        sender: OneshotMpscSenderHandle<String>,
    ) -> Result<usize, SendError<Lifecycle>> {
        self.lifecycle_channel_handle
            .send(Redirect(url.to_string(), sender))
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

    pub fn core_config(&self) -> &Config {
        self.manager.core_config()
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
                Err(_) => {
                    error!("Error while receiving signal in abort_on_stop.");
                }
            }
        }
    })
}

#[cfg(test)]
pub mod test {
    use super::*;

    use crate::domain::config::tests::test_config;

    pub fn get_test_manager(config: &Arc<Config>) -> Manager {
        let lifecycle_handle = LifecycleChannelHandle::new();
        Manager::new(config, lifecycle_handle)
    }
    mod readonly_manager {
        use super::*;
        use crate::domain::mpsc_handle::one_shot;
        use tokio_test::assert_ok;

        #[tokio::test]
        async fn test_abort_on_stop_with_init_and_stop() {
            let config = test_config();
            let mut manager = get_test_manager(&config);
            let readonly_manager = manager.readonly();
            let task = task::spawn(async {});
            let stop_handle = readonly_manager.abort_on_stop(&task);
            let (sender, receiver) = one_shot();
            manager
                .send_init(sender)
                .expect("Could not send init signal.");
            manager.send_stop().expect("Could not send stop signal.");
            assert_ok!(stop_handle.await);
            assert_eq!(receiver.len().await, 1);
        }

        #[tokio::test]
        async fn test_abort_on_stop_with_stop() {
            let config = test_config();
            let mut manager = get_test_manager(&config);
            let readonly_manager = manager.readonly();
            let task = task::spawn(async {});
            let stop_handle = readonly_manager.abort_on_stop(&task);
            manager.send_stop().expect("Could not send stop signal.");
            assert_ok!(stop_handle.await);
        }

        #[tokio::test]
        async fn test_get_receiver() {
            let config = test_config();
            let mut manager = get_test_manager(&config);
            let readonly_manager = manager.readonly();
            let receiver = readonly_manager.get_receiver();
            assert_eq!(receiver.len(), 0);
            manager.send_stop().expect("Could not send stop signal.");
            assert_eq!(receiver.len(), 1);
            let (sender, _) = one_shot();
            manager
                .send_init(sender)
                .expect("Could not send init signal.");
            assert_eq!(receiver.len(), 2);
        }
    }

    #[tokio::test]
    async fn test_send_read_config() {
        let config = test_config();
        let mut manager = get_test_manager(&config);
        let receiver = manager.readonly().get_receiver();
        let result = manager.send_read_config::<u32>();
        assert!(result.is_ok());
        assert_eq!(receiver.len(), 1);
    }
}
