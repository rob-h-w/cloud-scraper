use crate::domain::config::Config;
use derive_getters::Getters;

#[derive(Debug, Getters)]
pub struct SiteState {
    cert_folder: String,
    cert_path: String,
    key_path: String,
    #[allow(dead_code)]
    site_folder: String,
}

impl SiteState {
    pub fn new(config: &Config) -> Self {
        let cert_folder = match config.domain_config().tls_config() {
            None => None,
            Some(tls_config) => tls_config.cert_location().as_ref(),
        }
        .map(|it| it as &str)
        .unwrap_or(config.site_folder())
        .to_string();
        let path_base = cert_folder.clone();

        Self {
            cert_folder,
            cert_path: path_base.clone() + "/cert.pem",
            key_path: path_base + "/key.pem",
            site_folder: config.site_folder().to_string(),
        }
    }
}
