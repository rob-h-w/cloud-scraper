use derive_builder::Builder;
use derive_getters::Getters;
use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use serde::{Deserialize, Serialize};

#[derive(Builder, Deserialize, Getters, Serialize)]
pub struct ApplicationSecret {
    client_id: String,
    client_secret: String,
    auth_uri: String,
    auth_provider_x509_cert_url: Option<String>,
    token_uri: String,
    redirect_uris: Vec<String>,
    project_id: Option<String>,
    client_email: Option<String>,
    client_x509_cert_url: Option<String>,
}

impl ApplicationSecret {
    pub(crate) fn to_client(&self) -> BasicClient {
        let auth_url = AuthUrl::new(self.auth_uri.clone())
            .unwrap_or_else(|e| panic!("Invalid auth URI: {} caused error {:?}", self.auth_uri, e));
        let token_url = TokenUrl::new(self.token_uri.clone()).unwrap_or_else(|e| {
            panic!("Invalid token URI: {} caused error {:?}", self.token_uri, e)
        });
        let redirect_uri = RedirectUrl::new(self.redirect_uris[0].clone()).unwrap_or_else(|e| {
            panic!(
                "Invalid redirect URI: {} caused error {:?}",
                self.redirect_uris[0], e
            )
        });
        BasicClient::new(
            ClientId::new(self.client_id.clone()),
            Some(ClientSecret::new(self.client_secret.clone())),
            auth_url,
            Some(token_url),
        )
        .set_redirect_uri(redirect_uri)
    }
}
