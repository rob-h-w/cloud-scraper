pub mod web;

use crate::server::auth::get_token_path;
use crate::server::OauthInstalledFlowDelegate;
use crate::static_init::error::{Error, IoErrorExt};
use hyper_util::client::legacy::connect::HttpConnector;
use yup_oauth2::authenticator::Authenticator;
use yup_oauth2::hyper_rustls::HttpsConnector;
use yup_oauth2::{ApplicationSecret, InstalledFlowAuthenticator, InstalledFlowReturnMethod};

pub(crate) async fn get_authenticator(
    delegate: OauthInstalledFlowDelegate,
    secret: &ApplicationSecret,
) -> Result<Authenticator<HttpsConnector<HttpConnector>>, Error> {
    let token_path = get_token_path()
        .await
        .map_err(|e| e.to_source_creation_builder_error())?;
    let auth = InstalledFlowAuthenticator::builder(
        secret.clone(),
        InstalledFlowReturnMethod::HTTPPortRedirect(delegate.redirect_port()),
    )
    .flow_delegate(Box::new(delegate))
    .persist_tokens_to_disk(token_path)
    .build()
    .await
    .map_err(|e| e.to_source_creation_builder_error())?;
    Ok(auth)
}
