use crate::domain::channel_handle::{ChannelHandle, Readonly};
use crate::domain::entity::Entity;
use crate::domain::stop_task::stop_task;
use std::fmt::Debug;
use std::time::Duration;
use tokio::task;
use tokio::time::sleep;
use uuid::Uuid;

#[derive(Debug)]
pub(crate) struct Source {
    channel_handle: ChannelHandle<Entity<Uuid>>,
    stop_handle: Readonly<bool>,
}

impl Source {
    pub(crate) fn new(stop_handle: Readonly<bool>) -> Self {
        Self {
            channel_handle: ChannelHandle::new(),
            stop_handle,
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

        let stop_task = stop_task(&self.stop_handle, &task);

        let (_task_result, _stop_result) = tokio::join!(task, stop_task);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tokio::task::JoinSet;

    #[tokio::test]
    async fn test_stub_source_run() {
        let mut stop_handle = ChannelHandle::new();
        let mut join_set = JoinSet::new();
        let mut source = Source::new(stop_handle.read_only());

        let mut read_receiver = source.get_readonly_channel_handle().get_receiver();
        let _source_run = join_set.spawn(async move { source.run().await });
        let _receive_future = join_set.spawn(async move {
            let event = read_receiver.recv().await.unwrap();
            let diff = Utc::now().timestamp_millis() - event.created_at().timestamp_millis();

            assert!(diff < 100);
        });
        let _stop_abort_handle = join_set.spawn(async move {
            stop_handle.send(true).unwrap();
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
