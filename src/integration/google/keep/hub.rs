use google_keep1::oauth2::hyper::client::Client;
use google_keep1::oauth2::hyper::client::HttpConnector;
use google_keep1::oauth2::hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use google_keep1::oauth2::ServiceAccountKey;
use google_keep1::Keep;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;

use crate::integration::google::keep::auth::{auth, Error};
use crate::static_init::singleton::{async_ginit, reset as reset_singleton};

pub(crate) type KeepHub = Keep<HttpsConnector<HttpConnector>>;

const SHARED_KEEP: Lazy<RwLock<Option<KeepHub>>> = Lazy::new(|| RwLock::new(None));

pub(crate) async fn hub(key: &ServiceAccountKey) -> Result<KeepHub, Error> {
    async_ginit(&SHARED_KEEP, || async move {
        Ok(Keep::new(
            Client::builder().build(
                HttpsConnectorBuilder::new()
                    .with_native_roots()
                    .map_err(|e| Error::BuilderError(e))?
                    .https_only()
                    .enable_all_versions()
                    .build(),
            ),
            auth(key).await?,
        ))
    })
    .await
}

async fn reset() {
    reset_singleton(&SHARED_KEEP).await;
}
