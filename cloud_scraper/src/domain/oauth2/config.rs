use crate::domain;
use crate::domain::oauth2::{ApplicationSecret, ApplicationSecretBuilder};
use async_trait::async_trait;
use log::debug;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::io;
use std::io::Error;
use std::path::Path;
use tokio::fs;

pub trait ConfigProperties {
    fn auth_provider_x509_cert_url(&self) -> &str;
    fn auth_uri(&self) -> &str;
    fn client_email(&self) -> Option<&str>;
    fn client_id(&self) -> &str;
    fn client_secret(&self) -> &str;
    fn client_x509_cert_url(&self) -> Option<&str>;
    fn project_id(&self) -> &str;
    fn redirect_uris(&self) -> &Vec<String>;
    fn token_uri(&self) -> &str;
}

pub trait Config:
    ConfigProperties + Clone + Debug + DeserializeOwned + PartialEq + Send + Sync + Serialize
{
    fn to_application_secret(self, _core_config: &domain::Config) -> ApplicationSecret {
        ApplicationSecretBuilder::default()
            .auth_provider_x509_cert_url(Some(self.auth_provider_x509_cert_url().into()))
            .auth_uri(self.auth_uri().to_string())
            .client_email(self.client_email().map(|s| s.to_string()))
            .client_id(self.client_id().to_string())
            .client_secret(self.client_secret().to_string())
            .client_x509_cert_url(self.client_x509_cert_url().map(|s| s.to_string()))
            .project_id(Some(self.project_id().to_string()))
            .redirect_uris(self.redirect_uris().to_vec())
            .token_uri(self.token_uri().to_string())
            .build()
            .unwrap_or_else(|e| {
                panic!("Error while building ApplicationSecret: {:?}", e);
            })
    }
}

#[async_trait]
pub trait PersistableConfig: Sized {
    async fn persist(&self, path: &Path) -> Result<(), Error>;
    async fn read_config(path: &Path) -> Result<Self, Error>;
}

#[async_trait]
impl<T> PersistableConfig for T
where
    T: Config,
{
    async fn persist(&self, path: &Path) -> Result<(), Error> {
        debug!("Config path: {:?}", path);
        let serialized = serde_yaml::to_string(self).map_err(|e| {
            Error::new(
                io::ErrorKind::InvalidData,
                format!("Could not serialize config file due to {:?}.", e),
            )
        })?;

        fs::write(&path, serialized).await?;

        Ok(())
    }

    async fn read_config(path: &Path) -> Result<Self, Error> {
        debug!("Config path: {:?}", path);
        let slice = fs::read(&path).await.map_err(|e| {
            debug!(
                "Could not read config file at {} due to {:?}.",
                path.display(),
                e
            );
            e
        })?;
        debug!("Read result: {:?}", slice);
        let config_query = serde_yaml::from_slice(&slice).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Could not parse config file due to {:?}.", e),
            )
        })?;
        debug!("Parse result: {:?}", config_query);
        Ok(config_query)
    }
}

#[macro_export]
macro_rules! declare_config_struct {
    ($struct:ident) => {
        #[derive(Builder, Clone, Debug, Deserialize, PartialEq, Serialize)]
        pub struct $struct {
            auth_uri: String,
            auth_provider_x509_cert_url: String,
            client_email: Option<String>,
            client_id: String,
            client_secret: String,
            client_x509_cert_url: Option<String>,
            project_id: String,
            redirect_uris: Vec<String>,
            token_uri: String,
        }

        impl Config for $struct {}
    };
}

#[macro_export]
macro_rules! implement_getter {
    ($name:ident, $type:ty) => {
        fn $name(&self) -> &$type {
            &self.$name
        }
    };
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use derive_builder::Builder;
    use serde::Deserialize;

    declare_config_struct!(TestConfig);
    impl ConfigProperties for TestConfig {
        implement_getter!(auth_uri, str);
        implement_getter!(auth_provider_x509_cert_url, str);
        fn client_email(&self) -> Option<&str> {
            self.client_email.as_ref().map(|s| s.as_str())
        }
        implement_getter!(client_id, str);
        implement_getter!(client_secret, str);
        fn client_x509_cert_url(&self) -> Option<&str> {
            self.client_x509_cert_url.as_ref().map(|s| s.as_str())
        }
        implement_getter!(project_id, str);
        implement_getter!(redirect_uris, Vec<String>);
        implement_getter!(token_uri, str);
    }

    #[test]
    fn test_config_properties() {
        let config = TestConfigBuilder::default()
            .auth_uri("auth_uri".to_string())
            .auth_provider_x509_cert_url("auth_provider_x509_cert_url".to_string())
            .client_email(Some("client_email".to_string()))
            .client_id("client_id".to_string())
            .client_secret("client_secret".to_string())
            .client_x509_cert_url(Some("client_x509_cert_url".to_string()))
            .project_id("project_id".to_string())
            .redirect_uris(vec!["redirect_uris".to_string()])
            .token_uri("token_uri".to_string())
            .build()
            .unwrap();
        assert_eq!(config.auth_uri(), "auth_uri");
        assert_eq!(
            config.auth_provider_x509_cert_url(),
            "auth_provider_x509_cert_url"
        );
        assert_eq!(config.client_email(), Some("client_email"));
        assert_eq!(config.client_id(), "client_id");
        assert_eq!(config.client_secret(), "client_secret");
        assert_eq!(config.client_x509_cert_url(), Some("client_x509_cert_url"));
        assert_eq!(config.project_id(), "project_id");
        assert_eq!(config.redirect_uris(), &vec!["redirect_uris".to_string()]);
        assert_eq!(config.token_uri(), "token_uri");
    }

    mod persistable_config {
        use crate::domain::oauth2::PersistableConfig;
        use tempfile;

        #[tokio::test]
        async fn test_persistence() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("config.yaml");
            let config = super::TestConfigBuilder::default()
                .auth_uri("auth_uri".to_string())
                .auth_provider_x509_cert_url("auth_provider_x509_cert_url".to_string())
                .client_email(Some("client_email".to_string()))
                .client_id("client_id".to_string())
                .client_secret("client_secret".to_string())
                .client_x509_cert_url(Some("client_x509_cert_url".to_string()))
                .project_id("project_id".to_string())
                .redirect_uris(vec!["redirect_uris".to_string()])
                .token_uri("token_uri".to_string())
                .build()
                .unwrap();
            let result = config.persist(&path).await;
            assert!(result.is_ok());

            let read_config = super::TestConfig::read_config(&path)
                .await
                .expect("Could not read config");
            assert_eq!(config, read_config);
        }
    }
}
