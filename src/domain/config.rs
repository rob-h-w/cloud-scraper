use crate::core::cli::ServeArgs;
use derive_builder::Builder;
use derive_getters::Getters;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use std::vec;
use url::Url;

const HTTP_PORT: u16 = 80;
pub const TLS_PORT: u16 = 443;
pub const DEFAULT_SITE_FOLDER: &str = ".site";
const LOCALHOST: &str = "http://localhost";

lazy_static! {
    static ref DEFAULT_DOMAIN_CONFIG: DomainConfig = Default::default();
}

#[derive(Builder, Clone, Debug, Deserialize, Getters, PartialEq, Serialize)]
pub struct Config {
    #[getter(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    domain_config: Option<DomainConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    #[getter(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_after: Option<u64>,
    #[getter(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    site_state_folder: Option<String>,
}

impl Config {
    pub(crate) fn new(serve_args: &ServeArgs) -> Arc<Self> {
        Arc::new(match serve_args.config.as_ref() {
            Some(config_file) => {
                let config_file =
                    std::fs::read_to_string(config_file).expect("Could not open $config_file");
                let config: Config =
                    serde_yaml::from_str(&config_file).expect("Could not parse config");
                config
                    .merge_exit_after(serve_args.exit_after)
                    .merge_port(serve_args.port)
            }
            None => Self {
                domain_config: None,
                email: None,
                exit_after: serve_args.exit_after,
                site_state_folder: None,
            }
            .merge_port(serve_args.port),
        })
    }

    pub fn with_all_properties(
        domain_config: Option<DomainConfig>,
        email: Option<String>,
        exit_after: Option<u64>,
        site_state_folder: Option<String>,
    ) -> Self {
        Self {
            domain_config,
            email,
            exit_after,
            site_state_folder,
        }
    }

    pub(crate) fn domain_config(&self) -> &DomainConfig {
        self.domain_config
            .as_ref()
            .unwrap_or(&DEFAULT_DOMAIN_CONFIG)
    }

    pub(crate) fn exit_after(&self) -> Option<Duration> {
        self.exit_after.map(Duration::from_secs)
    }

    fn merge_exit_after(mut self, exit_after: Option<u64>) -> Self {
        if exit_after.is_some() {
            self.exit_after = exit_after;
        }

        self
    }

    fn merge_port(mut self, port: Option<u16>) -> Self {
        if let Some(p) = port {
            let mut domain_config = self.domain_config().clone();
            let mut url = domain_config.url().clone();
            url.set_port(Some(p)).expect("Could not set port");
            domain_config.url = url.clone();
            self.domain_config = Some(domain_config);
        }

        self
    }

    pub(crate) fn port(&self) -> u16 {
        if let Some(port) = self.domain_config().url().port() {
            port
        } else if self.uses_tls() {
            TLS_PORT
        } else {
            HTTP_PORT
        }
    }

    pub(crate) fn redirect_uri(&self) -> String {
        self.domain_config()
            .url_in_use()
            .join("/auth/google")
            .expect("Could not join redirect URI")
            .to_string()
    }

    pub fn sanity_check(&self) -> Result<(), String> {
        let mut errors = vec![];

        if self.uses_tls() {
            if self.email().is_none() {
                errors.push(format!(
                    "{} uses HTTPS, but no email address was provided for certificate requests",
                    self.domain_config().url()
                ));
            }

            if let Some(tls) = self.domain_config().tls_config() {
                if tls.builder_contacts().is_empty() {
                    errors.push("No builder contacts configured".to_string());
                }
            } else {
                errors.push("No TLS config found".to_string());
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join(", "))
        }
    }

    pub(crate) fn site_folder(&self) -> &str {
        match self.site_state_folder {
            Some(ref folder) => folder.as_str(),
            None => DEFAULT_SITE_FOLDER,
        }
    }

    pub(crate) fn uses_tls(&self) -> bool {
        self.domain_config().url_in_use().scheme() == "https"
    }

    pub(crate) fn websocket_url(&self) -> Url {
        let mut url = self
            .domain_config()
            .url_in_use()
            .join("/ws")
            .expect("Could not join websocket URL");
        url.set_scheme(self.ws_scheme())
            .expect("Could not set websocket scheme");
        url
    }

    fn ws_scheme(&self) -> &str {
        if self.uses_tls() {
            "wss"
        } else {
            "ws"
        }
    }
}

#[derive(Builder, Clone, Debug, Deserialize, Getters, PartialEq, Serialize)]
pub struct DomainConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    external_url: Option<Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tls_config: Option<TlsConfig>,
    #[serde(skip_serializing_if = "url_is_default")]
    url: Url,
}

fn url_is_default(url: &Url) -> bool {
    url.as_str() == LOCALHOST
}

impl Default for DomainConfig {
    fn default() -> Self {
        Self {
            external_url: None,
            tls_config: Some(TlsConfig {
                builder_contacts: vec![],
                poll_attempts: 3,
                poll_interval_seconds: 30,
            }),
            url: Url::parse(LOCALHOST).expect("Could not parse default URL"),
        }
    }
}

impl DomainConfig {
    pub fn new(url: &str) -> Self {
        Self {
            external_url: None,
            tls_config: None,
            url: Url::parse(url).unwrap_or_else(|_err| {
                panic!("Could not parse URL: {}", url);
            }),
        }
    }

    pub(crate) fn builder_contacts(&self) -> Vec<String> {
        self.tls_config()
            .as_ref()
            .expect("TLS config not defined")
            .builder_contacts()
            .clone()
    }

    pub(crate) fn poll_attempts(&self) -> usize {
        *self
            .tls_config()
            .as_ref()
            .expect("TLS config not defined")
            .poll_attempts()
    }

    pub(crate) fn poll_interval_seconds(&self) -> u64 {
        *self
            .tls_config()
            .as_ref()
            .expect("TLS config not defined")
            .poll_interval_seconds()
    }

    pub(crate) fn url_in_use(&self) -> Url {
        self.external_url().as_ref().unwrap_or(self.url()).clone()
    }
}

#[derive(Builder, Clone, Debug, Deserialize, Getters, PartialEq, Serialize)]
pub struct TlsConfig {
    builder_contacts: Vec<String>,
    poll_attempts: usize,
    poll_interval_seconds: u64,
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn test_config() -> Arc<Config> {
        Arc::new(Config {
            domain_config: Default::default(),
            email: None,
            exit_after: None,
            site_state_folder: None,
        })
    }

    pub(crate) fn test_config_with(
        domain_config: Option<DomainConfig>,
        email: Option<String>,
    ) -> Arc<Config> {
        Arc::new(Config {
            domain_config: Some(domain_config.unwrap_or(Default::default())),
            email,
            exit_after: None,
            site_state_folder: None,
        })
    }

    mod config {
        use super::*;

        #[test]
        fn test_instantiate() {
            let config = Config {
                domain_config: Default::default(),
                email: None,
                exit_after: None,
                site_state_folder: Some("test_site_folder".to_string()),
            };

            assert!(config.sanity_check().is_ok());
        }

        #[test]
        fn test_config_sanity_check() {
            let domain_config = Some(DomainConfig {
                external_url: Some(
                    Url::parse("https://the.domain:2222").expect("Could not parse URL"),
                ),
                tls_config: Some(TlsConfig {
                    builder_contacts: vec!["builder@contact.com".to_string()],
                    poll_attempts: 0,
                    poll_interval_seconds: 0,
                }),
                url: Url::parse("http://the.domain").expect("Could not parse URL"),
            });

            let bad_config = test_config_with(domain_config.clone(), None);
            assert!(bad_config.sanity_check().is_err());

            let config = test_config_with(domain_config, Some("the@email.com".to_string()));
            assert!(config.sanity_check().is_ok());
        }

        mod redirect_uri {
            use super::*;

            #[test]
            fn with_domain_config_returns_https() {
                let config = Config::with_all_properties(
                    Some(DomainConfig::new("http://test_domain:8080")),
                    None,
                    None,
                    Some("test".to_string()),
                );
                let redirect_uri = config.redirect_uri();
                assert_eq!(redirect_uri, "http://test_domain:8080/auth/google");
            }

            #[test]
            fn without_domain_config_returns_http() {
                let config =
                    Config::with_all_properties(None, None, None, None).merge_port(Some(8080));
                let redirect_uri = config.redirect_uri();
                assert_eq!(redirect_uri, "http://localhost:8080/auth/google");
            }
        }

        mod websocket_uri {
            use super::*;

            #[test]
            fn with_domain_config_returns_https() {
                let config = Config::with_all_properties(
                    Some(DomainConfig::new("https://test_domain:8080")),
                    None,
                    None,
                    None,
                );
                let websocket_uri = config.websocket_url().to_string();
                assert_eq!(websocket_uri, "wss://test_domain:8080/ws");
            }

            #[test]
            fn without_domain_config_returns_http() {
                let config: Config = Config::with_all_properties(None, None, None, None);
                let websocket_uri = config.websocket_url().to_string();
                assert_eq!(websocket_uri, "ws://localhost/ws");
            }
        }
    }
}
