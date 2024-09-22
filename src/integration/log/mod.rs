use crate::domain;
use crate::domain::channel_handle::Readonly;
use crate::domain::entity::Entity;
use crate::domain::node::{Manager, ReadonlyManager};
use log::info;
use std::fmt::Debug;
use tokio::join;
use tokio::sync::OwnedSemaphorePermit;
use tokio::task;
use uuid::Uuid;

#[derive(Debug)]
pub(crate) struct Sink {
    manager: ReadonlyManager,
    stub_receiver: Readonly<Entity<Uuid>>,
}

impl Sink {
    pub(crate) fn new(manager: &Manager, stub_receiver: &Readonly<Entity<Uuid>>) -> Self {
        Self {
            manager: manager.readonly(),
            stub_receiver: stub_receiver.clone(),
        }
    }

    pub(crate) async fn run(&mut self, log_permit: OwnedSemaphorePermit) {
        let mut stub_receiver = self.stub_receiver.get_receiver();
        let task = task::spawn(async move {
            loop {
                if let Ok(entity) = stub_receiver.recv().await {
                    info!("{}", stringify(entity));
                } else {
                    break;
                }
            }
        });

        let stop_handle = self.manager.abort_on_stop(&task).await;

        drop(log_permit);

        let (_task_result, _stop_result) = join!(task, stop_handle);
    }
}

fn stringify<T: domain::entity_data::EntityData>(entity: Entity<T>) -> String {
    match serde_yaml::to_string(&entity) {
        Ok(stringified) => stringified,
        Err(e) => {
            format!(
                "Error {} while stringifying entity with id: {}",
                e,
                entity.id()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_on;
    use crate::domain::config::tests::test_config;
    use crate::domain::node::LifecycleChannelHandle;
    use crate::integration::stub::Source;
    use crate::tests::Logger;
    use log::trace;
    use std::sync::Arc;
    use tokio::sync::Semaphore;
    use tokio::task::JoinSet;

    #[test]
    fn test_log_sink() {
        Logger::init();
        Logger::use_in(|logger| {
            logger.reset();

            async fn do_the_async_stuff() {
                let config = test_config();
                let lifecycle_channel_handle = LifecycleChannelHandle::new();
                let mut manager = Manager::new(&config, lifecycle_channel_handle);
                let stub_source = Source::new(&manager);

                let mut sink = Sink::new(&manager, &stub_source.get_readonly_channel_handle());

                let mut stub_source_handle = stub_source.get_channel_handle();

                let mut join_set = JoinSet::new();

                let semaphore = Arc::new(Semaphore::new(1));
                let permit = semaphore
                    .clone()
                    .acquire_owned()
                    .await
                    .expect("Could not acquire semaphore");

                let _abort_handle = join_set.spawn(async move { sink.run(permit).await });

                let _permit = semaphore
                    .acquire()
                    .await
                    .expect("Could not acquire semaphore");

                stub_source_handle
                    .send(Entity::new_now(Uuid::new_v4(), "1"))
                    .unwrap();
                stub_source_handle
                    .send(Entity::new_now(Uuid::new_v4(), "2"))
                    .unwrap();

                manager.send_stop().unwrap();

                while let Some(result) = join_set.join_next().await {
                    result.expect("Error in run_future.");
                }
            }

            block_on!(do_the_async_stuff());

            let log_entries = logger.log_entries();

            trace!("log entries: {:?}", log_entries);

            assert_eq!(log_entries.len(), 2);
        });
    }
}
