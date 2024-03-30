extern crate google_keep1 as keep1;

use std::io;

use google_keep1::oauth2::authenticator::Authenticator;
use google_keep1::oauth2::hyper::client::HttpConnector;
use google_keep1::oauth2::hyper_rustls::HttpsConnector;
use google_keep1::oauth2::ServiceAccountKey;
use keep1::oauth2;

#[derive(Debug)]
pub(crate) enum Error {
    BuilderError(io::Error),
}

pub(crate) async fn auth(
    key: &ServiceAccountKey,
) -> Result<Authenticator<HttpsConnector<HttpConnector>>, Error> {
    let auth = oauth2::ServiceAccountAuthenticator::builder(key.clone())
        .build()
        .await
        .map_err(|e| Error::BuilderError(e))?;
    Ok(auth)
}
