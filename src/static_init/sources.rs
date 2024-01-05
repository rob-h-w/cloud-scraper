use std::error::Error;

use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::domain::config::Config;
use crate::domain::source::Source;
use crate::domain::source_identifier::SourceIdentifier;
use crate::integration::stub::source::StubSource;

#[derive(Debug, EnumIter)]
pub(crate) enum Sources {
    Stub(Option<StubSource>),
}

impl Sources {
    pub(crate) fn identifier(&self) -> &SourceIdentifier {
        match self {
            Sources::Stub(instance) => instance.as_ref().unwrap().this_identifier(),
        }
    }
}

pub(crate) fn create_sources<ConfigType>(config: &ConfigType) -> Vec<Sources>
where
    ConfigType: Config,
{
    Sources::iter()
        .flat_map(|source_type| match source_type {
            Sources::Stub(_instance) => optional_init(config, StubSource::identifier(), || {
                Ok(Sources::Stub(Some(StubSource::new())))
            }),
        })
        .collect()
}

fn optional_init<ConfigType, Closure>(
    config: &ConfigType,
    source_identifier: &SourceIdentifier,
    initializer: Closure,
) -> Option<Sources>
where
    ConfigType: Config,
    Closure: Fn() -> Result<Sources, Box<dyn Error>>,
{
    if !config.source_configured(source_identifier.unique_name()) {
        None
    } else {
        Some(
            initializer().expect(
                format!(
                    "Failed to initialize source {src}",
                    src = source_identifier.unique_name()
                )
                .as_str(),
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::any::{Any, TypeId};
    use std::rc::Rc;

    use crate::core::config::Config as CoreConfig;
    use crate::domain::config::tests::TestConfig;
    use crate::integration::stub::source::StubSource;

    use super::*;

    #[test]
    fn test_create_sources_with_empty_config() {
        let config = Rc::new(TestConfig::new(None));

        let sources = create_sources(config.as_ref());

        assert_eq!(sources.len(), 0);
    }

    #[test]
    fn test_create_sources_with_stub_config() {
        let config = CoreConfig::new();

        let sources = create_sources(config.as_ref());

        assert!(sources.len() > 0);

        let enum_wrapper = sources.iter().find(|it| match *it {
            Sources::Stub(_) => true,
        });
        assert!(enum_wrapper.is_some());

        let Sources::Stub(instance) = enum_wrapper.unwrap();
        assert!(instance.is_some());

        let stub_source = instance.as_ref().unwrap();
        assert_eq!(stub_source.type_id(), TypeId::of::<StubSource>());
    }
}
