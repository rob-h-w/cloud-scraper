use std::collections::HashMap;

use crate::domain::config::Config;
use crate::domain::identifiable_sink::IdentifiableSink;
use crate::integration::log::sink::LogSink;

#[derive(Debug)]
pub(crate) enum Sinks {
    Log(LogSink),
}

pub(crate) fn create_sinks<ConfigType>(config: &ConfigType) -> HashMap<&str, Sinks>
where
    ConfigType: Config,
{
    let mut sinks = HashMap::new();

    for sink_name in config.sink_names() {
        match sink_name.as_str() {
            LogSink::SINK_ID => {
                sinks.insert(LogSink::SINK_ID, Sinks::Log(LogSink::new()));
            }
            _ => {}
        }
    }

    if config.sink_configured("log") {
        sinks.insert(LogSink::SINK_ID, Sinks::Log(LogSink::new()));
    }

    sinks
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
        let log = sinks.get("log").unwrap();
        assert!(match log {
            Sinks::Log(_) => true,
            #[allow(unreachable_patterns)]
            _ => false,
        });
    }
}
