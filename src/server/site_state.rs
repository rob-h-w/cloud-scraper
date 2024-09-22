use crate::domain::config::Config;

pub struct SiteState {
    cert_path: String,
    key_path: String,
    site_folder: String,
}

impl SiteState {
    pub fn new(config: &Config) -> Self {
        Self {
            cert_path: config.site_folder().to_string() + "/cert.pem",
            key_path: config.site_folder().to_string() + "/key.pem",
            site_folder: config.site_folder().to_string(),
        }
    }

    pub fn cert_path(&self) -> &str {
        &self.cert_path
    }

    pub fn key_path(&self) -> &str {
        &self.key_path
    }

    pub fn site_folder(&self) -> &str {
        &self.site_folder
    }
}
