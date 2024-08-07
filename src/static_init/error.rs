use serde_yaml::Value;
use std::io;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    BadServiceAccountKeyYaml(String),
    Builder(String),
    Connection(String),
    KeyNotFound(Value),
    NotAMapping(Value),
    YamlSerialization(String),
}

impl warp::reject::Reject for Error {}

pub trait SerdeErrorExt {
    fn to_bad_service_account_key_yaml_error(&self) -> Error;
    fn to_yaml_serialization_error(&self) -> Error;
}

impl SerdeErrorExt for serde_yaml::Error {
    fn to_bad_service_account_key_yaml_error(&self) -> Error {
        Error::BadServiceAccountKeyYaml(self.to_string())
    }

    fn to_yaml_serialization_error(&self) -> Error {
        Error::YamlSerialization(self.to_string())
    }
}

pub trait IoErrorExt {
    fn to_source_creation_builder_error(&self) -> Error;
}

impl IoErrorExt for io::Error {
    fn to_source_creation_builder_error(&self) -> Error {
        Error::Builder(self.to_string())
    }
}
