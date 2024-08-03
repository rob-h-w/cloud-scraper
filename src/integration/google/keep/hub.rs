use crate::integration::google::auth::get_authenticator;
use crate::static_init::error::{Error, IoErrorExt};
#[cfg(not(test))]
use crate::static_init::singleton::async_ginit;
#[cfg(test)]
use crate::static_init::singleton::{async_ginit, reset as reset_singleton};
use google_keep1::oauth2::hyper::client::Client;
use google_keep1::oauth2::hyper::client::HttpConnector;
use google_keep1::oauth2::hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use google_keep1::oauth2::ApplicationSecret;
use google_keep1::Keep;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;

pub(crate) type KeepHub = Keep<HttpsConnector<HttpConnector>>;

static SHARED_KEEP: Lazy<RwLock<Option<KeepHub>>> = Lazy::new(|| RwLock::new(None));

pub(crate) async fn hub(key: &ApplicationSecret) -> Result<KeepHub, Error> {
    async_ginit(&SHARED_KEEP, || async move {
        Ok(Keep::new(
            Client::builder().build(
                HttpsConnectorBuilder::new()
                    .with_native_roots()
                    .map_err(|e| e.to_source_creation_builder_error())?
                    .https_only()
                    .enable_all_versions()
                    .build(),
            ),
            get_authenticator(key).await?,
        ))
    })
    .await
}

#[cfg(test)]
async fn reset() {
    reset_singleton(&SHARED_KEEP).await;
}
