use std::time::Duration;
use std::vec;

use serde::Deserialize;

pub(crate) trait Config: 'static + Send + Sync {
    fn domain_config(&self) -> Option<&DomainConfig>;
    fn domain_is_defined(&self) -> bool {
        self.domain_config().is_some()
    }
    fn email(&self) -> Option<&str>;
    fn exit_after(&self) -> Option<Duration> {
        None
    }
    fn port(&self) -> u16;

    fn site_folder(&self) -> &str;

    fn sanity_check(&self) -> Result<(), String> {
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

#[derive(Clone, Debug, Deserialize)]
pub struct DomainConfig {
    pub builder_contacts: Vec<String>,
    pub domain_name: String,
    pub poll_attempts: usize,
    pub poll_interval_seconds: u64,
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn test_config_sanity_check() {
        let domain_config = Some(DomainConfig {
            builder_contacts: vec!["builder@contact.com".to_string()],
            domain_name: "the.domain".to_string(),
            poll_attempts: 0,
            poll_interval_seconds: 0,
        });

        let bad_config = TestConfig::new_domain_email(domain_config.clone(), None);
        assert!(bad_config.sanity_check().is_err());

        let config = TestConfig::new_domain_email(domain_config, Some("the@email.com".to_string()));
        assert!(config.sanity_check().is_ok());
    }

    pub(crate) struct TestConfig {
        domain_config: Option<DomainConfig>,
        email: Option<String>,
    }

    impl TestConfig {
        pub fn new_domain_email(
            domain_config: Option<DomainConfig>,
            email: Option<String>,
        ) -> Self {
            Self {
                domain_config,
                email,
            }
        }
    }

    impl Config for TestConfig {
        fn domain_config(&self) -> Option<&DomainConfig> {
            self.domain_config.as_ref()
        }

        fn email(&self) -> Option<&str> {
            self.email.as_deref()
        }

        fn port(&self) -> u16 {
            80
        }

        fn site_folder(&self) -> &str {
            "test_site_folder"
        }
    }
}
