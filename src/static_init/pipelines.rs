use std::collections::HashMap;
use std::fmt::Debug;

use crate::core::pipeline::{ExecutablePipeline, Pipeline};
use crate::domain::config::Config;
use crate::static_init::sinks::Sinks;
use crate::static_init::sources::Sources;
use crate::static_init::translators::{TranslatorCreationError, Translators};

pub(crate) fn create_pipelines<ConfigType>(
    config: &ConfigType,
    sources: &HashMap<&str, Sources>,
    sinks: &HashMap<&str, Sinks>,
) -> Result<Vec<Box<dyn ExecutablePipeline>>, PipelineBuildError>
where
    ConfigType: Config,
{
    let mut pipelines: Vec<Box<dyn ExecutablePipeline>> = Vec::new();

    for pipeline in config.pipelines() {
        let source = sources
            .get(pipeline.source())
            .ok_or(PipelineBuildError::MissingSource(
                pipeline.source().to_string(),
            ))?;
        let sink = sinks
            .get(pipeline.sink())
            .ok_or(PipelineBuildError::MissingSink(pipeline.sink().to_string()))?;
        let translator =
            Translators::new(sink, source).map_err(|e| PipelineBuildError::MissingTranslator(e))?;
        pipelines.push(with_translators(translator, source, sink)?);
    }

    Ok(pipelines)
}

#[derive(Debug)]
pub(crate) enum PipelineBuildError {
    MissingSink(String),
    MissingSource(String),
    MissingTranslator(TranslatorCreationError),
    UnsupportedSource(String),
}

impl PipelineBuildError {
    fn unsupported_source(source: &impl Debug, translator: &impl Debug) -> PipelineBuildError {
        PipelineBuildError::UnsupportedSource(format!(
            "Translator {:?} does not support source: {:?}",
            translator, source
        ))
    }
}

fn with_translators(
    translator: Translators,
    source: &Sources,
    sink: &Sinks,
) -> Result<Box<dyn ExecutablePipeline>, PipelineBuildError> {
    Ok(match translator {
        Translators::StringToString(translator) => match source {
            _ => return Err(PipelineBuildError::unsupported_source(source, &translator)),
        },
        Translators::UuidToString(translator) => match source {
            Sources::Stub(source) => match sink {
                Sinks::Log(sink) => Pipeline::new(source, translator, sink),
            },
            _ => return Err(PipelineBuildError::unsupported_source(source, &translator)),
        },
        Translators::GoogleKeepToString(translator) => match source {
            Sources::GoogleKeep(source) => match sink {
                Sinks::Log(sink) => Pipeline::new(source, translator, sink),
            },
            _ => return Err(PipelineBuildError::unsupported_source(source, &translator)),
        },
    })
}

#[cfg(test)]
mod tests {
    use crate::core::config::Config as CoreConfig;
    use crate::static_init::sinks::create_sinks;
    use crate::static_init::sources::create_sources;

    use super::*;

    #[test]
    fn get_translator_instantiates_when_no_direct_spec() {
        let config = CoreConfig::new_test();

        let sources = create_sources(config.as_ref()).expect("Failed to create sources");
        let sinks = create_sinks(config.as_ref());

        let pipelines = create_pipelines(config.as_ref(), &sources, &sinks)
            .expect("Failed to create pipelines");

        assert_eq!(pipelines.len(), 1);
    }
}
