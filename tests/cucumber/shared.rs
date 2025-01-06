use cloud_scraper::domain::{Config, DomainConfig};

pub fn test_config() -> Config {
    Config::with_all_properties(
        Some(DomainConfig::new("http://test.domain:8080")),
        Some("user@test.domain".to_string()),
        None,
        None,
    )
}
