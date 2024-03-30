use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use once_cell::sync::Lazy;

use crate::core::error::PipelineError;
use crate::domain::entity::Entity;
use crate::domain::entity_data::EntityData;
use crate::domain::entity_translator::EntityTranslator;
use crate::domain::sink::Sink;
use crate::domain::source::Source;

const START_TIME: Lazy<DateTime<Utc>> =
    Lazy::new(|| Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap());

pub(crate) type UpdatedAtVec = Vec<DateTime<Utc>>;

#[async_trait]
pub(crate) trait ExecutablePipeline: Send + Sync {
    async fn run(&self, since: Option<DateTime<Utc>>) -> Result<UpdatedAtVec, PipelineError>;
}

pub(crate) struct Pipeline<FromType, ToType, SourceType, TranslatorType, SinkType>
where
    FromType: EntityData,
    SinkType: Sink<ToType>,
    SourceType: Source<FromType>,
    ToType: EntityData,
    TranslatorType: EntityTranslator<FromType, ToType>,
{
    source: Arc<SourceType>,
    translator: TranslatorType,
    sink: Arc<SinkType>,
    phantom_from: std::marker::PhantomData<FromType>,
    phantom_to: std::marker::PhantomData<ToType>,
}

#[async_trait]
impl<FromType, ToType, SourceType, TranslatorType, SinkType> ExecutablePipeline
    for Pipeline<FromType, ToType, SourceType, TranslatorType, SinkType>
where
    FromType: EntityData,
    ToType: EntityData,
    SourceType: Source<FromType> + Send,
    TranslatorType: EntityTranslator<FromType, ToType>,
    SinkType: Sink<ToType>,
{
    async fn run(&self, since: Option<DateTime<Utc>>) -> Result<UpdatedAtVec, PipelineError> {
        let entities = self
            .source
            .get(&(if let Some(s) = since { s } else { *START_TIME }))
            .await
            .map_err(|e| PipelineError::Source(format!("Failed to retrieve entities: {:?}", e)))?;
        let translated_entities: Vec<Entity<ToType>> = entities
            .iter()
            .map(|entity| self.translator.translate(&entity))
            .collect();
        self.sink
            .put(&translated_entities)
            .await
            .map_err(|e| PipelineError::Sink(e.to_string()))?;
        Ok(translated_entities
            .iter()
            .map(|e| e.updated_at().clone())
            .collect())
    }
}

impl<FromType, ToType, SourceType, TranslatorType, SinkType>
    Pipeline<FromType, ToType, SourceType, TranslatorType, SinkType>
where
    FromType: EntityData,
    SinkType: Sink<ToType>,
    SourceType: Source<FromType>,
    ToType: EntityData,
    TranslatorType: EntityTranslator<FromType, ToType>,
{
    pub(crate) fn new(
        source: &Arc<SourceType>,
        translator: TranslatorType,
        sink: &Arc<SinkType>,
    ) -> Box<Self> {
        Box::new(Self {
            source: source.clone(),
            translator,
            sink: sink.clone(),
            phantom_from: Default::default(),
            phantom_to: Default::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;

    use crate::block_on;
    use crate::domain::entity_translator::tests::TestTranslator;
    use crate::domain::sink::tests::TestSink;
    use crate::integration::stub::source::StubSource;

    use super::*;

    #[test]
    fn test_dev_usability() {
        let source = Arc::new(StubSource::new());
        let translator = TestTranslator;
        let sink = Arc::new(TestSink {});
        let pipeline = Pipeline::new(&source, translator, &sink);
        let updates =
            block_on!(pipeline.run(Some(Utc::now() - TimeDelta::try_seconds(1).unwrap()))).unwrap();

        assert_eq!(updates.len(), 1)
    }
}
