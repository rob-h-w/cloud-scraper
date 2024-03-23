use crate::domain::entity::Entity;
use crate::domain::entity_user::EntityUser;
use crate::domain::identifiable_source::IdentifiableSource;
use crate::domain::source::Source;
use crate::integration::google::keep::google_keep::GoogleKeep;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_yaml::Value;
use std::any::TypeId;
use std::error::Error;

#[derive(Debug)]
pub(crate) struct KeepSource {}

impl KeepSource {
    pub(crate) fn new(_config: &Value) -> Self {
        Self {}
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
    async fn get(&self, _since: &DateTime<Utc>) -> Result<Vec<Entity<GoogleKeep>>, Box<dyn Error>> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {}
