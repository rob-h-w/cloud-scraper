use crate::integration::google::keep::translator::Translator as GoogleKeepTranslator;
use crate::integration::no_op_translator::NoOpTranslator;
use crate::integration::stub::translator::UuidToStringTranslator;
use crate::static_init::sinks::Sinks;
use crate::static_init::sources::Sources;
use crate::static_init::translators::TranslatorCreationError::MissingSinkType;

#[derive(Clone, Debug)]
pub(crate) enum Translators {
    StringToString(NoOpTranslator<String>),
    GoogleKeepToString(GoogleKeepTranslator),
    UuidToString(UuidToStringTranslator),
}

impl Translators {
    pub(crate) fn new(sink: &Sinks, source: &Sources) -> Result<Self, TranslatorCreationError> {
        match sink {
            Sinks::Log(_) => match source {
                Sources::Stub(_) => Ok(Translators::UuidToString(UuidToStringTranslator)),
                Sources::GoogleKeep(_) => Ok(Translators::GoogleKeepToString(GoogleKeepTranslator)),
            },
            _ => Err(MissingSinkType),
        }
    }
}

#[derive(Debug)]
pub(crate) enum TranslatorCreationError {
    MissingSinkType,
    MissingSourceType,
}
