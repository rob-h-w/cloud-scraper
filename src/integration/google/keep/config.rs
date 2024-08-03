use crate::static_init::error::{Error, SerdeErrorExt};
use google_keep1::oauth2::ApplicationSecret;
use serde::Deserialize;
use serde_yaml::Value;

use crate::core::serde_yaml::{FluentMutatorMappingExtension, MappingExtension};
use crate::integration::google::auth::web::ConfigQuery;

const LOCALHOST_PATH: &str = "http://localhost:8080/api/sessions/oauth/google";

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub web: ApplicationSecret,
}

impl Config {
    pub fn from_yaml(value: Value) -> Result<Self, Error> {
        let value = Self::set_default_redirect_uri(value)?;

        serde_yaml::from_value(Value::from(value))
            .map_err(|e| e.to_bad_service_account_key_yaml_error())
    }

    fn set_default_redirect_uri(value: Value) -> Result<Value, Error> {
        value.with_at_str(
            "web",
            &value.get_by_str("web")?.with_default_at_str(
                "redirect_uris",
                &Value::Sequence(vec![Value::String(LOCALHOST_PATH.to_string())]),
            ),
        )
    }
}

pub trait ConfigExtension {
    fn to_config(&self) -> Result<Config, Error>;
}

impl ConfigExtension for Value {
    fn to_config(&self) -> Result<Config, Error> {
        Config::from_yaml(self.clone())
    }
}

impl ConfigExtension for ConfigQuery {
    fn to_config(&self) -> Result<Config, Error> {
        Ok(Config {
            web: ApplicationSecret {
                client_id: self.client_id(),
                client_secret: self.client_secret(),
                token_uri: self.token_uri(),
                auth_uri: self.auth_uri(),
                redirect_uris: vec![],
                project_id: Some(self.project_id()),
                client_email: None,
                auth_provider_x509_cert_url: Some(self.auth_provider_x509_cert_url()),
                client_x509_cert_url: None,
            },
        })
    }
}
