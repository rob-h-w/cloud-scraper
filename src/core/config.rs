use std::sync::Arc;
use std::time::Duration;

use crate::core::cli::ServeArgs;
use serde::Deserialize;

use crate::domain::config::Config as DomainConfig;

const TLS_PORT: u16 = 443;
const DEFAULT_SITE_FOLDER: &str = ".site";

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    domain_config: Option<crate::domain::config::DomainConfig>,
    email: Option<String>,
    exit_after: Option<u64>,
    port: Option<u16>,
    site_state_folder: Option<String>,
}

impl Config {
    pub(crate) fn new(serve_args: &ServeArgs) -> Arc<Self> {
        Arc::new(match serve_args.config.as_ref() {
            Some(config_file) => {
                let config_file =
                    std::fs::read_to_string(config_file).expect("Could not open $config_file");
                let mut config: Config =
                    serde_yaml::from_str(&config_file).expect("Could not parse config");
                config.merge_exit_after(serve_args.exit_after);
                config.merge_port(serve_args.port);
                config
            }
            None => Self {
                domain_config: None,
                email: None,
                exit_after: serve_args.exit_after,
                port: serve_args.port,
                site_state_folder: None,
            },
        })
    }

    #[cfg(test)]
    pub(crate) fn new_test() -> Arc<Self> {
        Arc::new(Self {
            domain_config: None,
            email: None,
            exit_after: None,
            port: None,
            site_state_folder: None,
        })
    }

    fn merge_exit_after(&mut self, exit_after: Option<u64>) {
        if exit_after.is_some() {
            self.exit_after = exit_after;
        }
    }

    fn merge_port(&mut self, port: Option<u16>) {
        if let Some(p) = port {
            self.port = Some(p);
        }
    }
}

impl DomainConfig for Config {
    fn domain_config(&self) -> Option<&crate::domain::config::DomainConfig> {
        self.domain_config.as_ref()
    }

    fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    fn exit_after(&self) -> Option<Duration> {
        self.exit_after.map(Duration::from_secs)
    }

    fn port(&self) -> u16 {
        self.port.unwrap_or(TLS_PORT)
    }

    fn site_folder(&self) -> &str {
        match self.site_state_folder {
            Some(ref folder) => folder.as_str(),
            None => DEFAULT_SITE_FOLDER,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instantiate() {
        let config = Config {
            domain_config: None,
            email: None,
            exit_after: None,
            port: None,
            site_state_folder: None,
        };

        assert_eq!(config.port(), TLS_PORT);
        assert!(config.sanity_check().is_ok());
    }
}
