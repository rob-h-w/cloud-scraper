use cloud_scraper::domain::node::Manager;
use cloud_scraper::domain::oauth2::Client;
use cloud_scraper::domain::oauth2::{ApplicationSecret, ExtraParameters};
use cloud_scraper::server::WebEventChannelHandle;
use cloud_scraper::static_init::error::Error;
use oauth2::AccessToken;
use parking_lot::ReentrantMutex;
use std::fmt::{Debug, Formatter};
use std::path::Path;
use std::sync::Arc;

pub(crate) struct TestClientImpl {}

impl Debug for TestClientImpl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TestClientImpl")
    }
}

impl TestClientImpl {
    fn new_proxy(
        application_secret: ApplicationSecret,
        extra_parameters: &ExtraParameters,
        manager: &Manager,
        token_path: &Path,
        web_channel_handle: &WebEventChannelHandle,
    ) -> Self {
        Self {}
    }
    fn get_token_proxy(&self, scopes: &[String]) -> Result<AccessToken, Error> {
        Ok(AccessToken::new("token".to_string()))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TestClient {
    internal: Arc<ReentrantMutex<TestClientImpl>>,
}

impl Client for TestClient {
    fn new(
        application_secret: ApplicationSecret,
        extra_parameters: &ExtraParameters,
        manager: &Manager,
        token_path: &Path,
        web_channel_handle: &WebEventChannelHandle,
    ) -> Self {
        Self {
            internal: Arc::new(ReentrantMutex::new(TestClientImpl::new_proxy(
                application_secret,
                extra_parameters,
                manager,
                token_path,
                web_channel_handle,
            ))),
        }
    }

    async fn get_token(&self, scopes: &[&str]) -> Result<AccessToken, Error> {
        let scopes = scopes.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        self.internal.lock().get_token_proxy(&scopes)
    }
}
