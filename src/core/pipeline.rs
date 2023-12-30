use std::error::Error;
use std::fmt::Debug;

use chrono::{DateTime, Utc};

use crate::domain::entity::Entity;
use crate::domain::entity_translator::EntityTranslator;
use crate::domain::sink::Sink;
use crate::domain::source::Source;

struct Pipeline<'a, T, U, V, W, X>
where
    T: Debug + 'static,
    U: Debug + 'static,
    V: Source<T>,
    W: EntityTranslator<T, U>,
    X: Sink<U>,
{
    source: &'a mut V,
    translator: &'a W,
    sink: &'a mut X,
    phantom_t: std::marker::PhantomData<T>,
    phantom_u: std::marker::PhantomData<U>,
}

impl<'a, T, U, V, W, X> Pipeline<'a, T, U, V, W, X>
where
    T: Debug + 'static,
    U: Debug + 'static,
    V: Source<T>,
    W: EntityTranslator<T, U>,
    X: Sink<U>,
{
    fn new(source: &'a mut V, translator: &'a W, sink: &'a mut X) -> Box<Self> {
        Box::new(Self {
            source,
            translator,
            sink,
            phantom_t: Default::default(),
            phantom_u: Default::default(),
        })
    }

    fn run(&mut self, since: Option<DateTime<Utc>>) -> Result<usize, Box<dyn Error>> {
        let entities = self
            .source
            .get(&(if let Some(s) = since { s } else { Utc::now() }))?;
        let translated_entities: Vec<Entity<U>> = entities
            .iter()
            .map(|entity| self.translator.translate(&entity))
            .collect();
        self.sink.put(&translated_entities)?;
        Ok(translated_entities.len())
    }
}

#[cfg(test)]
mod tests {
    use crate::core::config::Config;
    use crate::domain::entity_translator::tests::TestTranslator;
    use crate::domain::entity_translator::EntityTranslator;
    use crate::domain::sink::tests::TestSink;
    use crate::integration::stub::source::StubSource;
    use chrono::Duration;

    use super::*;

    #[test]
    fn test_dev_usability() {
        let mut source = StubSource::new();
        let translator = TestTranslator::new(Config {});
        let mut sink = TestSink::new("test");
        let mut pipeline = Pipeline::new(&mut source, &translator, &mut sink);
        let count = pipeline
            .run(Some(Utc::now() - Duration::seconds(1)))
            .unwrap();

        assert_eq!(count, 1)
    }
}
