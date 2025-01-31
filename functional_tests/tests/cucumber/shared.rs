use cloud_scraper::domain::{Config, DomainConfig};
use std::path::PathBuf;

pub(crate) fn test_config() -> Config {
    Config::with_all_properties(
        Some(DomainConfig::new("http://test.domain:8080")),
        Some("user@test.domain".to_string()),
        None,
        None,
    )
}

fn workspace() -> PathBuf {
    test_crate_root().join("..")
}

pub(crate) fn test_crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub(crate) fn cs_home_dir() -> PathBuf {
    test_crate_root()
}

pub(crate) fn default_config_file() -> PathBuf {
    cs_home_dir().join("config.yaml")
}

pub(crate) fn bin_folder() -> PathBuf {
    workspace().join("target").join("debug")
}
