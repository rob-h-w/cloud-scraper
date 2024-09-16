mod manager;

pub use manager::{Manager, ReadonlyManager};

#[cfg(test)]
pub use manager::test::get_test_manager;

use crate::domain::channel_handle::ChannelHandle;
use crate::domain::mpsc_handle::OneshotMpscSenderHandle;
use crate::domain::node::Lifecycle::ReadConfig;
use async_trait::async_trait;
use std::any::TypeId;
use Lifecycle::Stop;

#[derive(Clone, Debug, PartialEq)]
pub enum Lifecycle {
    Init(OneshotMpscSenderHandle<()>),
    ReadConfig(TypeId),
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

#[cfg(test)]
pub mod test {
    use crate::domain::config::tests::test_config;
    use crate::domain::node::manager::test::get_test_manager;

    pub struct TestNode;

    mod readonly_manager {
        use super::*;
        use crate::domain::mpsc_handle::one_shot;
        use tokio::task;
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
