use crate::integration::google::keep::error::Error;
use google_keep1::oauth2::ServiceAccountKey;
use serde::Deserialize;
use serde_yaml::Value;

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub(crate) service_account_key: ServiceAccountKey,
}

impl Config {
    pub(crate) fn from_yaml(value: Value) -> Result<Self, Error> {
        serde_yaml::from_value(value).map_err(|e| Error::BadConfig(e))
    }
}
