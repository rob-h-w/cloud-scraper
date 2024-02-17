// Need an enum generated by macro from the source & sink enums.

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

use crate::core::pipeline::{ExecutablePipeline, Pipeline};
use crate::domain::config::{Config, TranslatorConfig};
use crate::domain::entity_data::EntityData;
use crate::domain::entity_translator::{EntityTranslator, TranslationDescription};
use crate::domain::entity_user::EntityUser;
use crate::domain::sink;
use crate::domain::sink::Sink as SinkTrait;
use crate::domain::source::Source;
use crate::integration::log::Sink;
use crate::integration::stub::source::StubSource;
use crate::static_init::sinks::Sinks;
use crate::static_init::sources::Sources;
use crate::static_init::translators::Translators::NoOp;
use crate::static_init::translators::{Translators, SUPPORTED_TYPES};

pub(crate) fn create_pipelines<ConfigType>(
    config: &ConfigType,
    sources: &[Sources],
    sinks: &HashMap<&str, Sinks>,
    translators: &HashMap<TranslationDescription, Translators>,
) -> Vec<Box<dyn ExecutablePipeline>>
where
    ConfigType: Config,
{
    let mut pipelines: Vec<Box<dyn ExecutablePipeline>> = Vec::new();

    for pipeline in config.pipelines() {
        let source = sources
            .iter()
            .find(|source| source.identifier().unique_name() == pipeline.source())
            .unwrap_or_else(|| panic!("Missing source: {}", pipeline.source()));
        let sink = sinks
            .get(pipeline.sink())
            .unwrap_or_else(|| panic!("Missing sink: {}", pipeline.sink()));
        let translator = translators
            .get_translator(pipeline.translator(), source, sink)
            .unwrap_or_else(|| {
                panic!(
                    "Missing translator from source: {from}, to sink: {to}",
                    from = pipeline.source(),
                    to = pipeline.sink()
                )
            });
        pipelines.push(with_translators(translator, source, sink));
    }

    pipelines
}

fn with_translators(
    translator: &Translators,
    source: &Sources,
    sink: &Sinks,
) -> Box<dyn ExecutablePipeline> {
    match translator {
        NoOp => {
            let it: Box<dyn ExecutablePipeline> = noop_translator(source, sink);
            it
        }
        Translators::UuidToString(implementation) => {
            let it: Box<dyn ExecutablePipeline> = with_translator(
                implementation.as_ref().expect("Could not get translator"),
                source,
                sink,
            );
            it
        }
    }
}

fn with_translator<FromType, ToType, TranslatorType>(
    translator: &TranslatorType,
    sources: &Sources,
    sinks: &Sinks,
) -> Box<dyn ExecutablePipeline>
where
    FromType: EntityData,
    StubSource: Source<FromType>,
    ToType: EntityData,
    TranslatorType: EntityTranslator<FromType, ToType>,
    Sink: sink::Sink<ToType>,
{
    match sources {
        Sources::Stub(implementation) => translator_with_source(
            translator,
            implementation.as_ref().expect("Could not get source"),
            sinks,
        ),
    }
}

fn translator_with_source<FromType, SourceType, ToType, TranslatorType>(
    translator: &TranslatorType,
    source: &Arc<SourceType>,
    sink: &Sinks,
) -> Box<dyn ExecutablePipeline>
where
    FromType: EntityData,
    SourceType: Source<FromType>,
    ToType: EntityData,
    TranslatorType: EntityTranslator<FromType, ToType>,
    Sink: SinkTrait<ToType>,
{
    match sink {
        Sinks::Log(implementation) => translator_with_sink(translator, source, implementation),
    }
}

fn translator_with_sink<FromType, SinkType, SourceType, ToType, TranslatorType>(
    translator: &TranslatorType,
    source: &Arc<SourceType>,
    sink: &Arc<SinkType>,
) -> Box<dyn ExecutablePipeline>
where
    FromType: EntityData,
    SinkType: SinkTrait<ToType>,
    SourceType: Source<FromType>,
    ToType: EntityData,
    TranslatorType: EntityTranslator<FromType, ToType>,
{
    Pipeline::new(source, translator, sink)
}

fn noop_translator(source: &Sources, _: &Sinks) -> Box<dyn ExecutablePipeline> {
    let could_not_get_source = "Could not get source";
    match source {
        Sources::Stub(_) => panic!("{}", could_not_get_source),
    }
}

fn noop_translator_with_source<DataType, SourceType>(
    source: &Arc<SourceType>,
    sinks: &Sinks,
) -> Box<dyn ExecutablePipeline>
where
    SourceType: Source<DataType> + Source<String>,
    DataType: EntityData,
    Sink: sink::Sink<DataType>,
{
    match sinks {
        Sinks::Log(implementation) => noop_translator_with_sink(source, implementation),
    }
}

fn noop_translator_with_sink<DataType, SinkType, SourceType>(
    source: &Arc<SourceType>,
    sink: &Arc<SinkType>,
) -> Box<dyn ExecutablePipeline>
where
    DataType: EntityData,
    SinkType: SinkTrait<DataType>,
    SourceType: Source<DataType>,
{
    Pipeline::new_no_op(source, sink)
}

trait TranslatorGetter {
    fn get_translator(
        &self,
        translator_spec: Option<&TranslatorConfig>,
        source: &Sources,
        sink: &Sinks,
    ) -> Option<&Translators>;
}

impl TranslatorGetter for HashMap<TranslationDescription, Translators> {
    fn get_translator(
        &self,
        translator_spec: Option<&TranslatorConfig>,
        source: &Sources,
        sink: &Sinks,
    ) -> Option<&Translators> {
        if let Some(translator_config) = translator_spec {
            let from_type_id = SUPPORTED_TYPES
                .get(translator_config.from())
                .expect("Could not find from type")
                .clone();
            let to_type_id = SUPPORTED_TYPES
                .get(translator_config.to())
                .expect("Could not find to type")
                .clone();
            let translation_description = TranslationDescription {
                from: from_type_id,
                to: to_type_id,
            };
            return self.get(&translation_description);
        }

        let from_list: Vec<TypeId> = match source {
            Sources::Stub(instance) => instance.as_ref().unwrap().this_supports_entity_data(),
        };

        let to_list: Vec<TypeId> = match sink {
            Sinks::Log(instance) => instance.this_supports_entity_data(),
        };

        if from_list
            .iter()
            .find(|from| to_list.contains(from))
            .is_some()
        {
            return Some(&NoOp);
        }

        for from in from_list.iter() {
            for to in to_list.iter() {
                let translation_description = TranslationDescription {
                    from: *from,
                    to: *to,
                };
                if let Some(translator) = self.get(&translation_description) {
                    return Some(translator);
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::core::config::Config as CoreConfig;
    use crate::static_init::sinks::create_sinks;
    use crate::static_init::sources::create_sources;
    use crate::static_init::translators::create_translators;

    #[test]
    fn get_translator_instantiates_when_no_direct_spec() {
        let config = CoreConfig::new();

        let sources = create_sources(config.as_ref());
        let sinks = create_sinks(config.as_ref());
        let translators = create_translators();

        let pipelines = create_pipelines(config.as_ref(), &sources, &sinks, &translators);

        assert_eq!(pipelines.len(), 1);
    }
}
