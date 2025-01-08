use crate::domain::oauth2::Config as Oauth2Config;
use crate::domain::oauth2::{
    make_config_struct, ApplicationSecret, ApplicationSecretBuilder, PersistableConfig,
};
use crate::domain::Config;
use derive_builder::Builder;
use log::debug;
use paste::paste;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::path::Path;
use tokio::fs;

macro_rules! make_config_query {
    ($struct:ident, { $($e:ident),* }, { $($d:ident, $v:literal),* }) => {
        make_config_struct!(
            $struct,
            Oauth2Config,
            { $($e),* },
            { $($d, $v),* }
        );
        paste! {
            impl $struct {
                pub(crate) fn empty_page_data() -> HashMap<&'static str, String> {
                    let mut page_data = HashMap::new();
                    $(
                        page_data.insert(stringify!($e), Self::format_empty(stringify!($e)));
                    )*
                    $(
                        page_data.insert(stringify!($d), Self::format(stringify!($d), $v));
                    )*
                    page_data
                }

                fn format(name: &str, value: &str) -> String {
                    format!("name=\"{}\" value=\"{}\"", name,  value)
                }

                fn format_empty(name: &str) -> String {
                    format!("name=\"{}\"", name)
                }

                pub(crate) fn to_page_data(&self) -> HashMap<&'static str, String> {
                    let mut page_data = HashMap::new();
                    $(
                        page_data.insert(stringify!($e), Self::format(stringify!($e), &self.$e));
                    )*
                    $(
                        page_data.insert(stringify!($d), Self::format(stringify!($d), &self.$d));
                    )*
                    page_data
                }
            }
        }
    }
}
make_config_query!(
    ConfigQuery,
    { project_id, client_id, client_secret },
    {
        auth_uri, "https://accounts.google.com/o/oauth2/auth",
        auth_provider_x509_cert_url, "https://www.googleapis.com/oauth2/v1/certs",
        token_uri, "https://oauth2.googleapis.com/token"
    }
);

impl PersistableConfig for ConfigQuery {
    async fn persist(&self, path: &Path) -> Result<(), io::Error> {
        debug!("Config path: {:?}", path);
        let serialized = serde_yaml::to_string(self).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Could not serialize config file due to {:?}.", e),
            )
        })?;

        fs::write(&path, serialized).await?;

        Ok(())
    }

    async fn read_config(path: &Path) -> Result<Self, io::Error> {
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

impl ConfigQuery {
    pub(crate) fn to_application_secret(&self, config: &Config) -> ApplicationSecret {
        ApplicationSecretBuilder::default()
            .auth_provider_x509_cert_url(Some(self.auth_provider_x509_cert_url().into()))
            .auth_uri(self.auth_uri().into())
            .client_email(None)
            .client_id(self.client_id().into())
            .client_secret(self.client_secret().into())
            .client_x509_cert_url(None)
            .project_id(Some(self.project_id().into()))
            .redirect_uris(vec![config.redirect_uri()])
            .token_uri(self.token_uri().into())
            .build()
            .unwrap_or_else(|e| {
                panic!("Error while building ApplicationSecret: {:?}", e);
            })
    }
}
