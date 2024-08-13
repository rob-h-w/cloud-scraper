use crate::domain::mpsc_handle::one_shot;
use crate::domain::node::Manager;
use log::debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task;
use yup_oauth2::authenticator_delegate::InstalledFlowDelegate;

#[derive(Clone, Debug)]
pub struct OauthFlowDelegateFactory {
    node_manager: Manager,
}

impl OauthFlowDelegateFactory {
    pub fn new(manager: &Manager) -> Self {
        Self {
            node_manager: manager.clone(),
        }
    }

    pub fn get_installed_flow_delegate(&self) -> OauthInstalledFlowDelegate {
        OauthInstalledFlowDelegate::new(&self.node_manager)
    }
}

pub struct OauthInstalledFlowDelegate {
    node_manager: Manager,
}

impl OauthInstalledFlowDelegate {
    pub fn new(manager: &Manager) -> Self {
        Self {
            node_manager: manager.clone(),
        }
    }
}

impl InstalledFlowDelegate for OauthInstalledFlowDelegate {
    fn present_user_url<'a>(
        &'a self,
        url: &'a str,
        need_code: bool,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(present_user_url(&self.node_manager, url, need_code))
    }
}

async fn present_user_url(
    node_manager: &Manager,
    url: &str,
    need_code: bool,
) -> Result<String, String> {
    debug!("Presenting user url: {}", url);
    let (sender, mut receiver) = one_shot();
    let semaphore = Arc::new(Semaphore::new(1));
    let permit = semaphore
        .clone()
        .acquire_owned()
        .await
        .expect("Could not acquire semaphore");
    let task = task::spawn(async move {
        drop(permit);
        receiver.recv().await
    });
    let stop_task = node_manager.readonly().abort_on_stop(&task);
    let _permit = semaphore
        .clone()
        .acquire_owned()
        .await
        .expect("Could not acquire semaphore");

    node_manager
        .clone()
        .send_oauth2_redirect_url(url, need_code, sender)
        .map_err(|e| format!("Failed to send oauth2 redirect url: {}", e))?;

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
