use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::config::Config;
use crate::domain::identifiable_source::IdentifiableSource;
use crate::integration::google::keep::source::KeepSource;
use crate::integration::stub::source::StubSource;
use crate::static_init::sources::SourceCreationError::MissingImplementation;

#[derive(Debug)]
pub(crate) enum Sources {
    Stub(Arc<StubSource>),
    GoogleKeep(Arc<KeepSource>),
}

pub(crate) fn create_sources<ConfigType>(
    config: &ConfigType,
) -> Result<HashMap<&str, Sources>, SourceCreationError>
where
    ConfigType: Config,
{
    let mut sources = HashMap::new();

    for source_name in config.source_names() {
        match source_name.as_str() {
            KeepSource::SOURCE_ID => {
                sources.insert(
                    KeepSource::SOURCE_ID,
                    Sources::GoogleKeep(Arc::new(KeepSource::new(
                        config.source(KeepSource::SOURCE_ID).ok_or(
                            SourceCreationError::MissingConfig(KeepSource::SOURCE_ID.to_string()),
                        )?,
                    ))),
                );
            }
            StubSource::SOURCE_ID => {
                sources.insert(
                    StubSource::SOURCE_ID,
                    Sources::Stub(Arc::new(StubSource::new())),
                );
            }
            name => {
                return Err(MissingImplementation(name.to_string()));
            }
        }
    }

    Ok(sources)
}

#[derive(Debug)]
pub(crate) enum SourceCreationError {
    MissingConfig(String),
    MissingImplementation(String),
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use tokio_test::assert_err;

    use crate::core::config::Config as CoreConfig;
    use crate::domain::config::tests::TestConfig;

    use super::*;

    #[test]
    fn test_create_sources_with_empty_config() {
        let config = Rc::new(TestConfig::new(None));

        let result = create_sources(config.as_ref());

        let error = assert_err!(result);

        assert_eq!(
            format!("{:?}", error),
            "MissingImplementation(\"test source\")"
        )
    }

    #[test]
    fn test_create_sources_with_stub_config() {
        let config = CoreConfig::new_test();

        let sources = create_sources(config.as_ref()).expect("Failed to create sources");

        assert!(sources.len() > 0);
        assert!(sources.contains_key("stub"));

        let enum_wrapper = sources.get("stub");
        assert!(enum_wrapper.is_some());

        match enum_wrapper.unwrap() {
            Sources::Stub(_) => {}
            _ => panic!("Expected Sources::Stub"),
        }
    }
}
