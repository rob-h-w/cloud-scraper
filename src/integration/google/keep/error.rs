use std::io;

#[derive(Debug)]
pub enum Error {
    BadConfig(serde_yaml::Error),
    BadServiceAccountKeyYaml(serde_yaml::Error),
    BuilderError(io::Error),
    Unknown,
}
