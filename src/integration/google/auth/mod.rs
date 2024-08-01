pub mod web;

use crate::domain::module_state::NamedModule;
use crate::static_init::error::{Error, IoErrorExt};
use hyper_util::client::legacy::connect::HttpConnector;
use yup_oauth2::authenticator::Authenticator;
use yup_oauth2::hyper_rustls::HttpsConnector;
use yup_oauth2::ApplicationSecret;
use yup_oauth2::DeviceFlowAuthenticator;

pub(crate) struct Google {}

impl NamedModule for Google {
    fn name() -> &'static str {
        "google"
    }
}

pub(crate) async fn get_authenticator(
    secret: &ApplicationSecret,
) -> Result<Authenticator<HttpsConnector<HttpConnector>>, Error> {
    let auth = DeviceFlowAuthenticator::builder(secret.clone())
        .build()
        .await
        .map_err(|e| e.to_source_creation_builder_error())?;
    Ok(auth)
}
