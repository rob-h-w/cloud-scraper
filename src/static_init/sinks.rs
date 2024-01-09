use std::error::Error;

use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::domain::config::Config;
use crate::domain::identifiable_sink::IdentifiableSink;
use crate::domain::sink_identifier::SinkIdentifier;
use crate::integration::log::sink::LogSink;

#[derive(Debug, EnumIter)]
pub(crate) enum Sinks {
    Log(Option<LogSink>),
}

impl Sinks {
    pub(crate) fn identifier(&self) -> &SinkIdentifier {
        match self {
            Sinks::Log(instance) => instance.as_ref().unwrap().this_identifier(),
        }
    }
}

pub(crate) fn create_sinks<ConfigType>(config: &ConfigType) -> Vec<Sinks>
where
    ConfigType: Config,
{
    Sinks::iter()
        .flat_map(|sink_type| match sink_type {
            Sinks::Log(_instance) => optional_init(config, LogSink::identifier(), || {
                Ok(Sinks::Log(Some(LogSink::new())))
            }),
        })
        .collect()
}

fn optional_init<ConfigType, Closure>(
    config: &ConfigType,
    sink_identifier: &SinkIdentifier,
    initializer: Closure,
) -> Option<Sinks>
where
    ConfigType: Config,
    Closure: Fn() -> Result<Sinks, Box<dyn Error>>,
{
    if !config.sink_configured(sink_identifier.unique_name()) {
        None
    } else {
        Some(
            initializer().expect(
                format!(
                    "Failed to initialize sink {src}",
                    src = sink_identifier.unique_name()
                )
                .as_str(),
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::core::config::Config as CoreConfig;
    use crate::domain::config::tests::TestConfig;

    use super::*;

    #[test]
    fn test_create_sinks_with_empty_config() {
        let config = TestConfig::new(None);
        let sinks = create_sinks(&config);
        assert_eq!(sinks.len(), 0);
    }

    #[test]
    fn test_create_sinks_with_log_config() {
        let config = CoreConfig::new();
        let sinks = create_sinks(config.as_ref());
        assert!(sinks.len() > 0);
        assert!(sinks.iter().any(|sink| match sink {
            Sinks::Log(_) => true,
        }));
    }
}
