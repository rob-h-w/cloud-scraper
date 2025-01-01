use crate::domain::mpsc_handle::one_shot;
use crate::domain::node::Manager;
use crate::domain::oauth2::extra_parameters::{ExtraParameters, WithExtraParametersExt};
use crate::domain::oauth2::token::{BasicTokenResponseExt, Token, TokenExt, TokenStatus};
use crate::domain::oauth2::ApplicationSecret;
use crate::server::Event::Redirect;
use crate::server::{Code, Event, WebEventChannelHandle};
use crate::static_init::error::Error::FailedAfterRetries;
use crate::static_init::error::{
    Error, IoErrorExt, JoinErrorExt, RequestTokenErrorExt, SerdeErrorExt,
};
use log::{debug, error};
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::url::Url;
use oauth2::{
    AccessToken, AuthorizationRequest, CsrfToken, PkceCodeChallenge, PkceCodeVerifier,
    RefreshToken, Scope,
};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::Semaphore;
use tokio::task;
use tokio::time::sleep;
use Error::Oauth2CsrfMismatch;
use Event::Oauth2Code;

pub trait Client: Clone + Send + Sized + Sync + 'static {
    fn new(
        application_secret: ApplicationSecret,
        extra_parameters: &ExtraParameters,
        manager: &Manager,
        token_path: &Path,
        web_channel_handle: &WebEventChannelHandle,
    ) -> Self;
    fn get_token(
        &self,
        scopes: &[&str],
    ) -> impl Future<Output = Result<AccessToken, Error>> + Send + Sync;
}

#[derive(Clone)]
pub(crate) struct BasicClientImpl {
    basic_client: BasicClient,
    extra_parameters: ExtraParameters,
    manager: Manager,
    retry_max: u8,
    retry_period: std::time::Duration,
    token_path: PathBuf,
    web_channel_handle: WebEventChannelHandle,
}

impl Client for BasicClientImpl {
    fn new(
        application_secret: ApplicationSecret,
        extra_parameters: &ExtraParameters,
        manager: &Manager,
        token_path: &Path,
        web_channel_handle: &WebEventChannelHandle,
    ) -> Self {
        let basic_client = application_secret.to_client();
        Self {
            basic_client,
            extra_parameters: extra_parameters.clone(),
            manager: manager.clone(),
            retry_max: 9,
            retry_period: std::time::Duration::from_secs(2),
            token_path: token_path.to_owned(),
            web_channel_handle: web_channel_handle.clone(),
        }
    }

    async fn get_token(&self, scopes: &[&str]) -> Result<AccessToken, Error> {
        match self.get_token_status_from_file().await {
            TokenStatus::Ok(token) => Ok(token.access_token().clone()),
            TokenStatus::Expired(refresh_token) => self
                .refresh_token(&refresh_token)
                .await
                .map(|token| token.access_token().clone()),
            TokenStatus::Absent => self.retrieve_token(scopes).await.map(|token| {
                debug!("Token retrieved: {:?}", token);
                token.access_token().clone()
            }),
        }
    }
}

impl BasicClientImpl {
    async fn await_code(&self) -> Result<Code, Error> {
        let mut receiver = self.web_channel_handle.get_receiver();
        let mut attempts = self.retry_max + 1;
        let callback_path = Url::parse(
            self.basic_client
                .redirect_url()
                .expect("Redirect URL not set."),
        )
        .expect("Redirect URL not valid.")
        .path()
        .to_string();

        let task = task::spawn(async move {
            debug!("Waiting for callback code");
            loop {
                if attempts == 0 {
                    return Err(FailedAfterRetries);
                }

                match receiver.recv().await {
                    Ok(event) => match event {
                        Oauth2Code(code, path) => {
                            if path == callback_path {
                                debug!("Got code");
                                return Ok(code);
                            } else {
                                debug!("Skipping oauth2 code for path: {}", path);
                                continue;
                            }
                        }
                        _ => {
                            debug!("Ignoring non-oauth2 event");
                            continue;
                        }
                    },
                    Err(e) => match e {
                        RecvError::Closed => {
                            debug!("Channel closed");
                        }
                        RecvError::Lagged(skipped_count) => {
                            debug!("Skipped {} events", skipped_count);
                            continue;
                        }
                    },
                }

                attempts -= 1;
            }
        });

        let stop_task = self.manager.readonly().abort_on_stop(&task).await;
        let result = task.await.map_err(|e| e.to_error());
        stop_task.abort();
        result?
    }

    async fn get_token_status_from_file(&self) -> TokenStatus {
        fs::read_to_string(&self.token_path)
            .await
            .ok()
            .map(|s| serde_yaml::from_str::<Token>(&s).ok())
            .unwrap_or(None)
            .get_status()
    }

    async fn present_url(&self, url: &Url) -> Result<(), Error> {
        debug!("Presenting user url: {}", url);
        let (sender, mut receiver) = one_shot();
        let semaphore = Arc::new(Semaphore::new(1));
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("Could not acquire semaphore");
        debug!("Acquired semaphore");
        let task = task::spawn(async move {
            debug!("Dropping permit");
            drop(permit);
            debug!("Waiting for redirect url");
            receiver.recv().await
        });
        let stop_task = self.manager.readonly().abort_on_stop(&task).await;
        let _permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("Could not acquire semaphore");

        let mut attempts = self.retry_max + 1;

        loop {
            if attempts == 0 {
                debug!("Max retries reached");
                return Err(FailedAfterRetries);
            }

            debug!("Sending oauth2 redirect url");
            match self
                .web_channel_handle
                .clone()
                .send(Redirect(url.to_string(), sender.clone()))
            {
                Ok(subscriber_count) => {
                    debug!(
                        "Sent oauth2 redirect url to {} subscribers",
                        subscriber_count
                    );
                    break;
                }
                Err(e) => {
                    debug!("Failed to send oauth2 redirect url: {:?}", e);
                    sleep(self.retry_period).await;
                    attempts -= 1;
                }
            };
        }

        debug!("Waiting for task to complete");
        let result = task.await;
        stop_task.abort();
        match result {
            Ok(optional) => match optional {
                None => Err(Error::Oauth2CodeMissing),
                Some(_) => Ok(()),
            },
            Err(e) => Err(e.to_error()),
        }
    }

    async fn refresh_token(&self, refresh_token: &RefreshToken) -> Result<Token, Error> {
        let token_status = self
            .basic_client
            .exchange_refresh_token(refresh_token)
            .with_extra_parameters(&self.extra_parameters)
            .request_async(async_http_client)
            .await
            .map_err(|e| e.to_error())?
            .to_token_status()
            .with_refresh_token(refresh_token);

        self.write_token(&token_status).await
    }

    fn make_redirect_url(&self, scopes: &[&str]) -> (PkceCodeVerifier, Url, CsrfToken) {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let mut request: AuthorizationRequest<'_> = self
            .basic_client
            .authorize_url(CsrfToken::new_random)
            .with_extra_parameters(&self.extra_parameters);

        for scope in scopes.iter() {
            request = request.add_scope(Scope::new(scope.to_string()));
        }

        let (redirect_url, csrf_state) = request.set_pkce_challenge(pkce_challenge).url();

        (pkce_verifier, redirect_url, csrf_state)
    }

    async fn retrieve_token(&self, scopes: &[&str]) -> Result<Token, Error> {
        let (pkce_verifier, redirect_url, csrf_state) = self.make_redirect_url(scopes);

        let code_future = self.await_code();
        self.present_url(&redirect_url).await?;

        let code = code_future.await?;

        if code.state().secret() != csrf_state.secret() {
            debug!("CSRF state secret did not match");
            return Err(Oauth2CsrfMismatch);
        }

        let token_status = self
            .basic_client
            .exchange_code(code.code().clone())
            .set_pkce_verifier(pkce_verifier)
            .request_async(async_http_client)
            .await
            .map_err(|e| e.to_error())?
            .to_token_status();

        self.write_token(&token_status).await
    }

    async fn write_token(&self, token_status: &TokenStatus) -> Result<Token, Error> {
        match token_status {
            TokenStatus::Ok(token) => {
                fs::write(
                    &self.token_path,
                    serde_yaml::to_string(token).map_err(|e| e.to_yaml_serialization_error())?,
                )
                .await
                .map_err(|e| e.to_error())?;
                Ok(token.clone())
            }
            TokenStatus::Expired(_) => {
                error!("Token was retrieved expired.");
                Err(Error::Oauth2TokenExpired)
            }
            TokenStatus::Absent => {
                error!("Token was not retrieved.");
                Err(Error::Oauth2TokenAbsent)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod make_redirect_url {
        use super::*;
        use crate::domain::config::tests::test_config;
        use crate::domain::node::get_test_manager;
        use crate::domain::oauth2::{extra_parameters, ApplicationSecretBuilder};

        #[test]
        fn gets_right_parameters_in_redirect_url() {
            let app_secret = ApplicationSecretBuilder::default()
                .auth_provider_x509_cert_url(None)
                .auth_uri("http://localhost:41047".to_string())
                .client_email(None)
                .client_id("client_id".to_string())
                .client_secret("client_secret".to_string())
                .client_x509_cert_url(None)
                .project_id(None)
                .redirect_uris(vec!["http://localhost:41047".to_string()])
                .token_uri("http://localhost:41047".to_string())
                .build()
                .unwrap();
            let client = BasicClientImpl::new(
                app_secret,
                &extra_parameters!("access_type" => "offline"),
                &get_test_manager(&test_config()),
                &Path::new("/test/path"),
                &WebEventChannelHandle::new(),
            );

            let (_pkce_verifier, redirect_url, _csrf_state) =
                client.make_redirect_url(&["scope1", "scope2"]);

            let url = redirect_url.as_str();
            assert!(
                url.starts_with("http://localhost:41047/?"),
                "expected '{}' to start with 'http://localhost:41047/?'",
                url
            );
            assert!(
                url.contains("response_type=code"),
                "expected '{}' to have 'response_type=code'",
                url
            );
            assert!(
                url.contains("client_id=client_id"),
                "expected '{}' to have 'client_id=client_id'",
                url
            );
            assert!(
                url.contains("state="),
                "expected '{}' to have 'state='",
                url
            );
            assert!(
                url.contains("code_challenge="),
                "expected '{}' to have 'code_challenge='",
                url
            );
            assert!(
                url.contains("code_challenge_method=S256"),
                "expected '{}' to have 'code_challenge_method=S256'",
                url
            );
            assert!(
                url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A41047"),
                "expected '{}' to have 'redirect_uri=http%3A%2F%2Flocalhost%3A41047'",
                url
            );
            assert!(
                url.contains("scope=scope1+scope2"),
                "expected scope=scope1+scope2 in '{}'",
                url
            );
            assert!(
                url.contains("access_type=offline"),
                "expected access_type=offline in '{}'",
                url
            );
        }
    }
}
