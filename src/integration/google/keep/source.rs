use crate::domain::channel_handle::ChannelHandle;
use crate::domain::entity::Entity;
use crate::domain::node::{Lifecycle, Manager, ReadonlyManager};
use crate::integration::google::auth::web::get_config;
use crate::integration::google::keep::config::{Config, ConfigExtension};
use crate::integration::google::keep::google_keep::GoogleKeep;
use crate::integration::google::keep::hub::hub;
use crate::integration::google::keep::Events;
use crate::static_init::error::Error;
use google_keep1::api::ListNotesResponse;
use google_keep1::client;
use google_keep1::hyper::{Body, Response};
use log::{error, info, trace};
use std::fmt::Debug;
use tokio::task;

type KeepResult = Result<Vec<Entity<GoogleKeep>>, Error>;

#[derive(Debug)]
pub(crate) struct Source {
    control_events: ChannelHandle<Events>,
    event_source_channel: ChannelHandle<Entity<GoogleKeep>>,
    lifecycle_manager: ReadonlyManager,
}

impl Source {
    pub(crate) fn new(lifecycle_channel_handle: &Manager) -> Self {
        Self {
            control_events: ChannelHandle::new(),
            event_source_channel: ChannelHandle::new(),
            lifecycle_manager: lifecycle_channel_handle.readonly(),
        }
    }

    pub fn control_events(&self) -> &ChannelHandle<Events> {
        &self.control_events
    }

    pub async fn run(&self) {
        let config_handle = ChannelHandle::new();

        let mut config_changed = config_handle.get_receiver();
        let event_sender = self.event_source_channel.clone();
        let task = task::spawn(async move {
            loop {
                match config_changed.recv().await {
                    Ok(config) => {
                        log::debug!("Got config: {:?}", config);
                        match Self::on_config(config, event_sender.clone()).await {
                            Ok(_) => {
                                info!("Note retrieval exited gracefully");
                            }
                            Err(e) => {
                                error!("Error while retrieving notes: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error while receiving config: {}", e);
                        break;
                    }
                }
            }
        });

        let mut lifecycle = self.lifecycle_manager.get_receiver();
        let config_ready = config_handle.clone();
        let task_abort_handle = task.abort_handle();
        let lifecycle_task = task::spawn(async move {
            loop {
                match lifecycle.recv().await {
                    Ok(lifecycle_event) => match lifecycle_event {
                        Lifecycle::Init(one_shot_handle) => {
                            // Bootstrap with a check for config.
                            Self::check_for_config(&mut config_handle.clone()).await;

                            one_shot_handle
                                .send(())
                                .await
                                .expect("Could not reply to init signal.");
                        }
                        Lifecycle::ReadConfig(type_id) => {
                            if type_id == std::any::TypeId::of::<Source>() {
                                Self::check_for_config(&mut config_ready.clone()).await;
                            }
                        }
                        Lifecycle::Stop => {
                            task_abort_handle.abort();
                            break;
                        }
                    },
                    Err(e) => {
                        error!("Error while receiving lifecycle event: {}", e);
                        break;
                    }
                }
            }
        });

        let (_lifecycle_task_result, _task) = tokio::join!(lifecycle_task, task);
    }

    async fn check_for_config(config_ready: &mut ChannelHandle<Config>) {
        if let Some(config_query) = get_config().await {
            match config_query.to_config() {
                Ok(config) => match config_ready.send(config) {
                    Ok(_) => {
                        info!("Config sent");
                    }
                    Err(e) => {
                        error!("Error while sending config: {:?}", e);
                    }
                },
                Err(e) => {
                    error!("Error while converting config: {:?}", e);
                }
            }
        }
    }

    async fn on_config(
        config: Config,
        mut event_sender: ChannelHandle<Entity<GoogleKeep>>,
    ) -> Result<(), Error> {
        trace!("starting on_config");
        let keep_hub = hub(&config.web).await?;

        let _ = keep_hub
            .notes()
            .list()
            .doit()
            .await
            .to_keep_result()
            .map(|entities| {
                entities.iter().for_each(|entity| {
                    let _ = event_sender.send(entity.clone());
                });
            });

        trace!("ending on_config");
        Ok(())
    }
}

trait NotesResponseResultExtension {
    fn to_keep_result(&self) -> KeepResult;
}

impl NotesResponseResultExtension for client::Result<(Response<Body>, ListNotesResponse)> {
    fn to_keep_result(&self) -> KeepResult {
        match self {
            Ok((response, list_notes_response)) => {
                log::debug!("Got response: {:?}", response);
                log::debug!("Got list_notes_response: {:?}", list_notes_response);
                Ok(list_notes_response.to_entities())
            }
            Err(e) => Err(Error::Connection(e.to_string())),
        }
    }
}

trait ListNotesResponseExtension {
    fn to_entities(&self) -> Vec<Entity<GoogleKeep>>;
}

impl ListNotesResponseExtension for ListNotesResponse {
    fn to_entities(&self) -> Vec<Entity<GoogleKeep>> {
        self.notes
            .as_ref()
            .unwrap_or(&vec![])
            .iter()
            .filter(|note| note.title.is_some() && note.create_time.is_some())
            .map(|note| {
                let create_time = note.create_time.expect("Create time is missing");
                Entity::new(
                    create_time,
                    GoogleKeep {},
                    note.title.as_ref().expect("Title is missing"),
                    note.update_time.unwrap_or(create_time),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {}
