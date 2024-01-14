use std::error::Error;

use chrono::{DateTime, Utc};

use crate::domain::entity::Entity;
use crate::domain::entity_data::EntityData;
use crate::domain::entity_translator::EntityTranslator;
use crate::domain::sink::Sink;
use crate::domain::source::Source;
use crate::integration::no_op_translator::NoOpTranslator;

pub(crate) trait ExecutablePipeline {
    fn run(&self, since: Option<DateTime<Utc>>) -> Result<usize, Box<dyn Error>>;
}

pub(crate) struct Pipeline<'a, FromType, ToType, SourceType, TranslatorType, SinkType>
where
    FromType: EntityData,
    SinkType: Sink<ToType>,
    SourceType: Source<FromType>,
    ToType: EntityData,
    TranslatorType: EntityTranslator<FromType, ToType>,
{
    source: &'a SourceType,
    translator: TranslatorType,
    sink: &'a SinkType,
    phantom_from: std::marker::PhantomData<FromType>,
    phantom_to: std::marker::PhantomData<ToType>,
}

impl<'a, FromType, ToType, SourceType, TranslatorType, SinkType> ExecutablePipeline
    for Pipeline<'a, FromType, ToType, SourceType, TranslatorType, SinkType>
where
    FromType: EntityData,
    ToType: EntityData,
    SourceType: Source<FromType>,
    TranslatorType: EntityTranslator<FromType, ToType>,
    SinkType: Sink<ToType>,
{
    fn run(&self, since: Option<DateTime<Utc>>) -> Result<usize, Box<dyn Error>> {
        let entities = self
            .source
            .get(&(if let Some(s) = since { s } else { Utc::now() }))?;
        let translated_entities: Vec<Entity<ToType>> = entities
            .iter()
            .map(|entity| self.translator.translate(&entity))
            .collect();
        self.sink.put(&translated_entities)?;
        Ok(translated_entities.len())
    }
}

impl<'a, DataType, SourceType, SinkType>
    Pipeline<'a, DataType, DataType, SourceType, NoOpTranslator, SinkType>
where
    DataType: EntityData,
    SourceType: Source<DataType>,
    SinkType: Sink<DataType>,
{
    pub(crate) fn new_no_op(source: &'a SourceType, sink: &'a SinkType) -> Box<Self> {
        Box::new(Self {
            source,
            sink,
            phantom_from: Default::default(),
            phantom_to: Default::default(),
            translator: NoOpTranslator::new(source),
        })
    }
}

impl<'a, FromType, ToType, SourceType, TranslatorType, SinkType>
    Pipeline<'a, FromType, ToType, SourceType, TranslatorType, SinkType>
where
    FromType: EntityData,
    SinkType: Sink<ToType>,
    SourceType: Source<FromType>,
    ToType: EntityData,
    TranslatorType: EntityTranslator<FromType, ToType>,
{
    pub(crate) fn new(
        source: &'a SourceType,
        translator: TranslatorType,
        sink: &'a SinkType,
    ) -> Box<Self> {
        Box::new(Self {
            source,
            translator,
            sink,
            phantom_from: Default::default(),
            phantom_to: Default::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use crate::core::config::Config;
    use crate::domain::entity_translator::tests::TestTranslator;
    use crate::domain::entity_translator::EntityTranslator;
    use crate::domain::sink::tests::TestSink;
    use crate::integration::stub::source::StubSource;

    use super::*;

    #[test]
    fn test_dev_usability() {
        let source = StubSource::new();
        let translator = TestTranslator::new(Config::new());
        let sink = TestSink {};
        let pipeline = Pipeline::new(&source, translator, &sink);
        let count = pipeline
            .run(Some(Utc::now() - Duration::seconds(1)))
            .unwrap();

        assert_eq!(count, 1)
    }
}
