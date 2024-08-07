pub mod web;

extern crate google_keep1 as keep1;

use crate::domain::module_state::NamedModule;
use crate::static_init::error::{Error, IoErrorExt};
use google_keep1::oauth2::authenticator::Authenticator;
use google_keep1::oauth2::hyper::client::HttpConnector;
use google_keep1::oauth2::hyper_rustls::HttpsConnector;
use google_keep1::oauth2::ApplicationSecret;
use keep1::oauth2;

pub(crate) struct Google {}

impl NamedModule for Google {
    fn name() -> &'static str {
        "google"
    }
}

pub(crate) async fn get_authenticator(
    secret: &ApplicationSecret,
) -> Result<Authenticator<HttpsConnector<HttpConnector>>, Error> {
    let auth = oauth2::InstalledFlowAuthenticator::builder(
        secret.clone(),
        oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .build()
    .await
    .map_err(|e| e.to_source_creation_builder_error())?;
    Ok(auth)
}
