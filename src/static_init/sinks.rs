use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::config::Config;
use crate::integration::*;

#[derive(Debug)]
pub(crate) enum Sinks {
    Log(Arc<log::Sink>),
}

pub(crate) fn create_sinks<ConfigType>(_config: &ConfigType) -> HashMap<&str, Sinks>
where
    ConfigType: Config,
{
    let sinks = HashMap::new();

    sinks
}

#[cfg(test)]
mod tests {
    use crate::domain::config::tests::TestConfig;

    use super::*;

    #[test]
    fn test_create_sinks_with_empty_config() {
        let config = TestConfig::new_domain_email(None, None);
        let sinks = create_sinks(&config);
        assert_eq!(sinks.len(), 0);
    }
}
