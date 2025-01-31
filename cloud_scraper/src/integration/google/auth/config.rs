use crate::domain::oauth2::{Config, ConfigProperties};
use derive_builder::Builder;
use derive_getters::Getters;
use serde::{Deserialize, Serialize};

#[derive(Builder, Clone, Debug, Deserialize, Getters, PartialEq, Serialize)]
pub struct ConfigQuery {
    #[builder(default=String::from("https://accounts.google.com/o/oauth2/auth"))]
    auth_uri: String,
    #[builder(default=String::from("https://www.googleapis.com/oauth2/v1/certs"))]
    auth_provider_x509_cert_url: String,
    client_email: Option<String>,
    client_id: String,
    client_secret: String,
    client_x509_cert_url: Option<String>,
    project_id: String,
    redirect_uris: Vec<String>,
    #[builder(default=String::from("https://oauth2.googleapis.com/token"))]
    token_uri: String,
}

impl ConfigProperties for ConfigQuery {
    fn auth_provider_x509_cert_url(&self) -> &str {
        &self.auth_provider_x509_cert_url
    }

    fn auth_uri(&self) -> &str {
        &self.auth_uri
    }

    fn client_email(&self) -> Option<&str> {
        self.client_email.as_deref()
    }

    fn client_id(&self) -> &str {
        &self.client_id
    }

    fn client_secret(&self) -> &str {
        &self.client_secret
    }

    fn client_x509_cert_url(&self) -> Option<&str> {
        self.client_x509_cert_url.as_deref()
    }

    fn project_id(&self) -> &str {
        &self.project_id
    }

    fn redirect_uris(&self) -> &Vec<String> {
        &self.redirect_uris
    }

    fn token_uri(&self) -> &str {
        &self.token_uri
    }
}

impl Config for ConfigQuery {}

impl ConfigQuery {
    pub fn new(
        auth_uri: String,
        auth_provider_x509_cert_url: String,
        client_email: Option<String>,
        client_id: String,
        client_secret: String,
        client_x509_cert_url: Option<String>,
        project_id: String,
        redirect_uris: Vec<String>,
        token_uri: String,
    ) -> Self {
        Self {
            auth_uri,
            auth_provider_x509_cert_url,
            client_email,
            client_id,
            client_secret,
            client_x509_cert_url,
            project_id,
            redirect_uris,
            token_uri,
        }
    }
}
