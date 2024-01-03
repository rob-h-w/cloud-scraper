use std::error::Error;
use std::fmt::Debug;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::domain::entity::Entity;
use crate::domain::entity_translator::EntityTranslator;
use crate::domain::sink::Sink;
use crate::domain::source::Source;

pub(crate) trait ExecutablePipeline {
    fn run(&mut self, since: Option<DateTime<Utc>>) -> Result<usize, Box<dyn Error>>;
}

struct Pipeline<'a, FromType, ToType, SourceType, TranslatorType, SinkType>
where
    FromType: Debug + 'static,
    ToType: Debug + 'static,
    SourceType: Source<FromType>,
    TranslatorType: EntityTranslator<FromType, ToType>,
    SinkType: Sink,
{
    source: &'a mut SourceType,
    translator: &'a TranslatorType,
    sink: &'a mut SinkType,
    phantom_from: std::marker::PhantomData<FromType>,
    phantom_to: std::marker::PhantomData<ToType>,
}

impl<'a, FromType, ToType, SourceType, TranslatorType, SinkType> ExecutablePipeline
    for Pipeline<'a, FromType, ToType, SourceType, TranslatorType, SinkType>
where
    FromType: Debug + 'static,
    ToType: Debug + Serialize + 'static,
    SourceType: Source<FromType>,
    TranslatorType: EntityTranslator<FromType, ToType>,
    SinkType: Sink,
{
    fn run(&mut self, since: Option<DateTime<Utc>>) -> Result<usize, Box<dyn Error>> {
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

impl<'a, FromType, ToType, SourceType, TranslatorType, SinkType>
    Pipeline<'a, FromType, ToType, SourceType, TranslatorType, SinkType>
where
    FromType: Debug + 'static,
    ToType: Debug + 'static,
    SourceType: Source<FromType>,
    TranslatorType: EntityTranslator<FromType, ToType>,
    SinkType: Sink,
{
    fn new(
        source: &'a mut SourceType,
        translator: &'a TranslatorType,
        sink: &'a mut SinkType,
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
        let mut source = StubSource::new();
        let translator = TestTranslator::new(Config::new());
        let mut sink = TestSink::new("test");
        let mut pipeline = Pipeline::new(&mut source, translator.as_ref(), &mut sink);
        let count = pipeline
            .run(Some(Utc::now() - Duration::seconds(1)))
            .unwrap();

        assert_eq!(count, 1)
    }
}
