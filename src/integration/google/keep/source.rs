use crate::domain::entity::Entity;
use crate::domain::entity_user::EntityUser;
use crate::domain::identifiable_source::IdentifiableSource;
use crate::domain::source::Source;
use crate::integration::google::keep::config::Config;
use crate::integration::google::keep::google_keep::GoogleKeep;
use crate::integration::google::keep::hub::{hub, KeepHub};
use crate::static_init::sources::{SourceCreationError, SourceExecutionError};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use debug_ignore::DebugIgnore;
use google_keep1::api::ListNotesResponse;
use google_keep1::client;
use google_keep1::hyper::{Body, Response};
use serde_yaml::Value;
use std::any::TypeId;
use std::error::Error;
use std::fmt::Debug;
use tokio::sync::Mutex;

type KeepResult = Result<Vec<Entity<GoogleKeep>>, Box<dyn Error>>;

#[derive(Debug)]
pub(crate) struct KeepSource {
    config: Config,
    hub: DebugIgnore<Mutex<Option<KeepHub>>>,
}

impl KeepSource {
    pub(crate) fn new(config: &Value) -> Result<Self, SourceCreationError> {
        Ok(Self {
            config: Config::from_yaml(config.clone())?,
            hub: Mutex::from(None).into(),
        })
    }

    async fn on_hub(&self) -> KeepResult {
        let mut lock = self.hub.lock().await;
        if lock.as_ref().is_none() {
            lock.replace(hub(&self.config.service_account_key).await.map_err(|e| {
                SourceExecutionError::SourceConnectionFailure(format!(
                    "Could not connect to source: {:?}",
                    e
                ))
            })?);
        }

        lock.as_ref()
            .unwrap()
            .notes()
            .list()
            .page_size(10)
            .doit()
            .await
            .to_keep_result()
    }
}

impl EntityUser for KeepSource {
    fn supported_entity_data() -> TypeId
    where
        Self: Sized,
    {
        TypeId::of::<GoogleKeep>()
    }
}

impl IdentifiableSource for KeepSource {
    const SOURCE_ID: &'static str = "google_keep";
}

#[async_trait]
impl Source<GoogleKeep> for KeepSource {
    async fn get(&self, _since: &DateTime<Utc>) -> KeepResult {
        self.on_hub().await
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
            Err(e) => Err(Box::new(SourceExecutionError::SourceConnectionFailure(
                e.to_string(),
            ))),
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
                let create_time = note.create_time.as_ref().unwrap();
                Entity::new::<KeepSource>(
                    create_time,
                    Box::new(GoogleKeep {}),
                    &note.title.as_ref().unwrap(),
                    &note.update_time.as_ref().unwrap_or(create_time),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {}
