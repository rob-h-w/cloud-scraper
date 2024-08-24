use crate::domain::mpsc_handle::one_shot;
use crate::domain::node::Manager;
use crate::server::Event::Redirect;
use crate::server::WebEventChannelHandle;
use log::debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task;
use tokio::time::sleep;
use yup_oauth2::authenticator_delegate::InstalledFlowDelegate;

#[derive(Clone, Debug)]
pub struct OauthFlowDelegateFactory {
    node_manager: Manager,
    redirect_port: u16,
    redirect_uri: String,
    web_channel_handle: WebEventChannelHandle,
}

impl OauthFlowDelegateFactory {
    pub fn new(
        manager: &Manager,
        redirect_port: u16,
        redirect_uri: &str,
        web_channel_handle: &WebEventChannelHandle,
    ) -> Self {
        Self {
            node_manager: manager.clone(),
            redirect_port,
            redirect_uri: redirect_uri.to_string(),
            web_channel_handle: web_channel_handle.clone(),
        }
    }

    pub fn get_installed_flow_delegate(&self) -> OauthInstalledFlowDelegate {
        OauthInstalledFlowDelegate::new(
            &self.node_manager,
            self.redirect_port,
            &self.redirect_uri,
            &self.web_channel_handle,
        )
    }
}

pub struct OauthInstalledFlowDelegate {
    node_manager: Manager,
    redirect_port: u16,
    redirect_uri: String,
    web_channel_handle: WebEventChannelHandle,
}

impl OauthInstalledFlowDelegate {
    pub(crate) fn redirect_port(&self) -> u16 {
        self.redirect_port
    }
}

impl OauthInstalledFlowDelegate {
    pub fn new(
        manager: &Manager,
        redirect_port: u16,
        redirect_uri: &str,
        web_channel_handle: &WebEventChannelHandle,
    ) -> Self {
        Self {
            node_manager: manager.clone(),
            redirect_uri: redirect_uri.to_string(),
            redirect_port,
            web_channel_handle: web_channel_handle.clone(),
        }
    }
}

impl InstalledFlowDelegate for OauthInstalledFlowDelegate {
    fn present_user_url<'a>(
        &'a self,
        url: &'a str,
        need_code: bool,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(present_user_url(
            &self.node_manager,
            &self.web_channel_handle,
            url,
            need_code,
        ))
    }

    fn redirect_uri(&self) -> Option<&str> {
        Some(&self.redirect_uri)
    }
}

async fn present_user_url(
    node_manager: &Manager,
    web_channel_handle: &WebEventChannelHandle,
    url: &str,
    need_code: bool,
) -> Result<String, String> {
    debug!("Presenting user url: {}, need_code: {}", url, need_code);
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
    let stop_task = node_manager.readonly().abort_on_stop(&task);
    let _permit = semaphore
        .clone()
        .acquire_owned()
        .await
        .expect("Could not acquire semaphore");

    loop {
        debug!("Sending oauth2 redirect url");
        match web_channel_handle
            .clone()
            .send(Redirect(url.to_string(), sender.clone()))
            .map_err(|e| format!("Failed to send oauth2 redirect url: {}", e))
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
                sleep(std::time::Duration::from_secs(1)).await;
            }
        };
    }

    debug!("Waiting for task to complete");
    let result = task.await;
    stop_task.abort();

    match result {
        Ok(url) => match url {
            Some(url) => Ok(url),
            None => Err("Failed to get oauth2 redirect url".to_string()),
        },
        Err(e) => Err(format!("Failed to get oauth2 redirect url: {:?}", e)),
    }
}
