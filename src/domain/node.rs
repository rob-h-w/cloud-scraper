use crate::domain::channel_handle::{ChannelHandle, Readonly};
use crate::domain::mpsc_handle::OneshotMpscSenderHandle;
use crate::domain::node::Lifecycle::{Init, ReadConfig};
use log::{error, trace};
use std::any::TypeId;
use tokio::sync::broadcast::error::SendError;
use tokio::sync::broadcast::Receiver;
use tokio::task;
use tokio::task::JoinHandle;
use Lifecycle::Stop;

#[derive(Clone, Debug, PartialEq)]
pub enum Lifecycle {
    Init(OneshotMpscSenderHandle<()>),
    ReadConfig(TypeId),
    Stop,
}

pub trait LifecycleAware {
    fn is_stop(&self) -> bool;
}

impl LifecycleAware for Lifecycle {
    fn is_stop(&self) -> bool {
        self == &Stop
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
}

impl<E: std::fmt::Debug> LifecycleAware for Result<Lifecycle, E> {
    fn is_stop(&self) -> bool {
        match self {
            Ok(lifecycle) => lifecycle.is_stop(),
            Err(e) => {
                panic!("Could not get lifecycle event because of {:?}", e)
            }
        }
    }
}

pub type LifecycleChannelHandle = ChannelHandle<Lifecycle>;
pub type ReadonlyLifecycleChannelHandle = Readonly<Lifecycle>;

#[derive(Clone, Debug)]
pub struct Manager {
    lifecycle_channel_handle: LifecycleChannelHandle,
}

impl Manager {
    pub fn new(lifecycle_channel_handle: LifecycleChannelHandle) -> Self {
        Self {
            lifecycle_channel_handle,
        }
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
    lifecycle_channel_handle: ReadonlyLifecycleChannelHandle,
}

impl ReadonlyManager {
    fn new(manager: &Manager) -> Self {
        Self {
            lifecycle_channel_handle: manager.lifecycle_channel_handle.read_only(),
        }
    }

    pub fn abort_on_stop<T>(&self, task: &JoinHandle<T>) -> JoinHandle<()> {
        abort_on_stop::<T>(self.lifecycle_channel_handle.get_receiver(), task)
    }

    pub fn get_receiver(&self) -> Receiver<Lifecycle> {
        self.lifecycle_channel_handle.get_receiver()
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
                    Init(event) => match event.send(()).await {
                        Ok(_) => {
                            trace!("Init signal sent in abort_on_stop.");
                        }
                        Err(_) => {
                            error!("Error while sending init signal in abort_on_stop.");
                        }
                    },
                    ReadConfig(_) => {}
                    Stop => {
                        task_abort_handle.abort();
                        break;
                    }
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

    pub fn get_test_manager() -> Manager {
        let lifecycle_handle = LifecycleChannelHandle::new();
        Manager::new(lifecycle_handle)
    }

    mod readonly_manager {
        use super::*;
        use crate::domain::mpsc_handle::one_shot;
        use tokio_test::assert_ok;

        #[tokio::test]
        async fn test_abort_on_stop_with_init_and_stop() {
            let mut manager = get_test_manager();
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
            let mut manager = get_test_manager();
            let readonly_manager = manager.readonly();
            let task = task::spawn(async {});
            let stop_handle = readonly_manager.abort_on_stop(&task);
            manager.send_stop().expect("Could not send stop signal.");
            assert_ok!(stop_handle.await);
        }

        #[tokio::test]
        async fn test_get_receiver() {
            let mut manager = get_test_manager();
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
        let mut manager = get_test_manager();
        let receiver = manager.readonly().get_receiver();
        let result = manager.send_read_config::<u32>();
        assert!(result.is_ok());
        assert_eq!(receiver.len(), 1);
    }
}
