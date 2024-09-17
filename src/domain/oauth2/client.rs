use crate::domain::mpsc_handle::one_shot;
use crate::domain::node::Manager;
use crate::domain::oauth2::ApplicationSecret;
use crate::server::Event::Redirect;
use crate::server::{Code, Event, WebEventChannelHandle};
use crate::static_init::error::Error::FailedAfterRetries;
use crate::static_init::error::{Error, JoinErrorExt, RequestTokenErrorExt};
use log::debug;
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::url::Url;
use oauth2::{AccessToken, CsrfToken, PkceCodeChallenge, Scope, TokenResponse};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::Semaphore;
use tokio::task;
use tokio::time::sleep;
use Error::Oauth2CsrfMismatch;
use Event::Oauth2Code;

pub(crate) struct Client {
    basic_client: BasicClient,
    manager: Manager,
    retry_max: u8,
    retry_period: std::time::Duration,
    token_path: PathBuf,
    web_channel_handle: WebEventChannelHandle,
}

impl Client {
    pub(crate) fn new(
        application_secret: ApplicationSecret,
        manager: &Manager,
        token_path: &PathBuf,
        web_channel_handle: &WebEventChannelHandle,
    ) -> Self {
        let basic_client = application_secret.to_client();
        Self {
            basic_client,
            manager: manager.clone(),
            retry_max: 9,
            retry_period: std::time::Duration::from_secs(2),
            token_path: token_path.clone(),
            web_channel_handle: web_channel_handle.clone(),
        }
    }

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

        let stop_task = self.manager.readonly().abort_on_stop(&task);
        let result = task.await.map_err(|e| e.to_error());
        stop_task.abort();
        result.unwrap_or_else(|e| Err(e))
    }

    pub(crate) async fn get_token(&self, scopes: &[&str]) -> Result<AccessToken, Error> {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let mut request = self.basic_client.authorize_url(CsrfToken::new_random);

        for scope in scopes.iter() {
            request = request.add_scope(Scope::new(scope.to_string()));
        }

        let (redirect_url, csrf_state) = request.set_pkce_challenge(pkce_challenge).url();

        let code_future = self.await_code();
        self.present_url(&redirect_url).await?;

        let code = code_future.await?;

        if code.state().secret() != csrf_state.secret() {
            debug!("CSRF state secret did not match");
            return Err(Oauth2CsrfMismatch);
        }

        let token_result = self
            .basic_client
            .exchange_code(code.code().clone())
            .set_pkce_verifier(pkce_verifier)
            .request_async(async_http_client)
            .await
            .map_err(|e| e.to_error())?;

        Ok(token_result.access_token().clone())
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
        let stop_task = self.manager.readonly().abort_on_stop(&task);
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
}
