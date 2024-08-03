use crate::domain::channel_handle::{ChannelHandle, Readonly};
use crate::domain::node::Lifecycle::ReadConfig;
use std::any::TypeId;
use tokio::sync::broadcast::error::SendError;
use tokio::sync::broadcast::Receiver;
use tokio::task;
use tokio::task::JoinHandle;
use Lifecycle::Stop;

#[derive(Clone, Debug, PartialEq)]
pub enum Lifecycle {
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

    pub fn load_config<T: 'static>(&mut self) -> Result<usize, SendError<Lifecycle>> {
        self.lifecycle_channel_handle
            .send(ReadConfig(TypeId::of::<T>()))
    }

    pub fn readonly(&self) -> ReadonlyManager {
        ReadonlyManager::new(&self)
    }

    pub fn stop(&mut self) -> Result<usize, SendError<Lifecycle>> {
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
        let mut stop_receiver = self.lifecycle_channel_handle.get_receiver();
        let task_abort_handle = task.abort_handle();
        task::spawn(async move {
            if let Ok(lifecycle_event) = stop_receiver.recv().await {
                if lifecycle_event.is_stop() {
                    task_abort_handle.abort();
                }
            }
        })
    }

    pub fn get_receiver(&self) -> Receiver<Lifecycle> {
        self.lifecycle_channel_handle.get_receiver()
    }
}
