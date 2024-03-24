extern crate google_keep1 as keep1;

use std::io;

use google_keep1::oauth2::authenticator::Authenticator;
use google_keep1::oauth2::hyper::client::HttpConnector;
use google_keep1::oauth2::hyper_rustls::HttpsConnector;
use keep1::oauth2;
use serde_yaml::Value;

#[derive(Debug)]
pub(crate) enum Error {
    BadServiceAccountKeyYaml(serde_yaml::Error),
    BuilderError(io::Error),
}

pub(crate) async fn auth(
    value: Value,
) -> Result<Authenticator<HttpsConnector<HttpConnector>>, Error> {
    let service_account_key: oauth2::ServiceAccountKey =
        serde_yaml::from_value(value.clone()).map_err(|e| Error::BadServiceAccountKeyYaml(e))?;
    let auth = oauth2::ServiceAccountAuthenticator::builder(service_account_key)
        .build()
        .await
        .map_err(|e| Error::BuilderError(e))?;
    Ok(auth)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn null_value_returns_error() {
        let result = auth(Value::Null).await;
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap(),
            Error::BadServiceAccountKeyYaml(_)
        ));
    }

    #[tokio::test]
    async fn invalid_value_returns_error() {
        let result = auth(Value::String("invalid".to_string())).await;
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap(),
            Error::BadServiceAccountKeyYaml(_)
        ));
    }
}
