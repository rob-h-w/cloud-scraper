use crate::core::cli::ServeArgs;
use derive_getters::Getters;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use std::vec;

const TLS_PORT: u16 = 443;
const DEFAULT_SITE_FOLDER: &str = ".site";

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    domain_config: Option<DomainConfig>,
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
    pub fn with_all_properties(
        domain_config: Option<DomainConfig>,
        email: Option<String>,
        exit_after: Option<u64>,
        port: Option<u16>,
        site_state_folder: Option<String>,
    ) -> Self {
        Self {
            domain_config,
            email,
            exit_after,
            port,
            site_state_folder,
        }
    }

    pub(crate) fn domain_config(&self) -> Option<&crate::domain::config::DomainConfig> {
        self.domain_config.as_ref()
    }

    pub fn domain_is_defined(&self) -> bool {
        self.domain_config.is_some()
    }

    pub fn domain_name(&self) -> &str {
        if let Some(domain_config) = self.domain_config() {
            &domain_config.domain_name
        } else {
            "localhost"
        }
    }

    fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    pub(crate) fn exit_after(&self) -> Option<Duration> {
        self.exit_after.map(Duration::from_secs)
    }

    pub(crate) fn port(&self) -> u16 {
        self.port.unwrap_or(TLS_PORT)
    }

    pub(crate) fn site_folder(&self) -> &str {
        match self.site_state_folder {
            Some(ref folder) => folder.as_str(),
            None => DEFAULT_SITE_FOLDER,
        }
    }

    pub(crate) fn uses_tls(&self) -> bool {
        self.port() == TLS_PORT || self.domain_is_defined()
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

    pub(crate) fn sanity_check(&self) -> Result<(), String> {
        let mut errors = vec![];

        if self.domain_is_defined() {
            if self.email().is_none() {
                errors.push("No email configured".to_string());
            }
        }

        if let Some(domain_config) = self.domain_config() {
            if domain_config.builder_contacts.is_empty() {
                errors.push("No builder contacts configured".to_string());
            }
            if domain_config.domain_name.is_empty() {
                errors.push("No domain name configured".to_string());
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join(", "))
        }
    }
}

#[derive(Clone, Debug, Deserialize, Getters)]
pub struct DomainConfig {
    pub builder_contacts: Vec<String>,
    pub domain_name: String,
    pub poll_attempts: usize,
    pub poll_interval_seconds: u64,
}

#[cfg(test)]
impl DomainConfig {
    pub fn new(domain_name: String) -> Self {
        Self {
            builder_contacts: vec![],
            domain_name,
            poll_attempts: 0,
            poll_interval_seconds: 0,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn test_config() -> Arc<Config> {
        Arc::new(Config {
            domain_config: None,
            email: None,
            exit_after: None,
            port: None,
            site_state_folder: None,
        })
    }

    pub fn test_config_with(
        domain_config: Option<DomainConfig>,
        email: Option<String>,
    ) -> Arc<Config> {
        Arc::new(Config {
            domain_config,
            email,
            exit_after: None,
            port: None,
            site_state_folder: None,
        })
    }

    mod config {
        use super::*;

        #[test]
        fn test_instantiate() {
            let config = Config {
                domain_config: None,
                email: None,
                exit_after: None,
                port: None,
                site_state_folder: Some("test_site_folder".to_string()),
            };

            assert_eq!(config.port(), TLS_PORT);
            assert!(config.sanity_check().is_ok());
        }

        #[test]
        fn test_config_sanity_check() {
            let domain_config = Some(DomainConfig {
                builder_contacts: vec!["builder@contact.com".to_string()],
                domain_name: "the.domain".to_string(),
                poll_attempts: 0,
                poll_interval_seconds: 0,
            });

            let bad_config = test_config_with(domain_config.clone(), None);
            assert!(bad_config.sanity_check().is_err());

            let config = test_config_with(domain_config, Some("the@email.com".to_string()));
            assert!(config.sanity_check().is_ok());
        }
    }
}
