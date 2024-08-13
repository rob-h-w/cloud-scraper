use crate::domain::channel_handle::{ChannelHandle, Readonly};
use crate::domain::entity::Entity;
use crate::domain::node::{Manager, ReadonlyManager};
use std::fmt::Debug;
use std::time::Duration;
use tokio::time::sleep;
use tokio::{join, task};
use uuid::Uuid;

#[derive(Debug)]
pub(crate) struct Source {
    channel_handle: ChannelHandle<Entity<Uuid>>,
    lifecycle_manager: ReadonlyManager,
}

impl Source {
    pub(crate) fn new(lifecycle_manager: &Manager) -> Self {
        Self {
            channel_handle: ChannelHandle::new(),
            lifecycle_manager: lifecycle_manager.readonly(),
        }
    }

    #[cfg(test)]
    pub fn get_channel_handle(&self) -> ChannelHandle<Entity<Uuid>> {
        self.channel_handle.clone()
    }

    pub fn get_readonly_channel_handle(&self) -> Readonly<Entity<Uuid>> {
        self.channel_handle.read_only()
    }

    pub(crate) async fn run(&mut self) {
        let mut sender = self.channel_handle.clone();
        let task = task::spawn(async move {
            loop {
                match sender.send(Entity::new_now(
                    Uuid::new_v4(),
                    &format!("stub {:?}", Uuid::new_v4()),
                )) {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("Error while sending: {}", e);
                    }
                }

                sleep(Duration::from_secs(1)).await;
            }
        });

        let stop_task = self.lifecycle_manager.abort_on_stop(&task);

        let (_task_result, _stop_result) = join!(task, stop_task);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::engine::tests::stub_config;
    use crate::domain::node::LifecycleChannelHandle;
    use chrono::Utc;
    use tokio::task::JoinSet;

    #[tokio::test]
    async fn test_stub_source_run() {
        let config = stub_config();
        let lifecycle_channel_handle = LifecycleChannelHandle::new();
        let manager = Manager::new(&config, lifecycle_channel_handle.clone());
        let mut join_set = JoinSet::new();
        let mut source = Source::new(&manager);

        let mut read_receiver = source.get_readonly_channel_handle().get_receiver();
        let _source_run = join_set.spawn(async move { source.run().await });
        let _receive_future = join_set.spawn(async move {
            let event = read_receiver.recv().await.unwrap();
            let diff = Utc::now().timestamp_millis() - event.created_at().timestamp_millis();

            assert!(diff < 100);
        });
        let mut stop_manager = manager.clone();
        let _stop_abort_handle = join_set.spawn(async move {
            stop_manager.send_stop().unwrap();
        });

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
