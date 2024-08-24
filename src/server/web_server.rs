use crate::server::{OauthFlowDelegateFactory, WebEventChannelHandle};

use crate::core::node_handles::NodeHandles;
use crate::domain::config::Config;
use crate::domain::node::{LifecycleAware, Manager};
use crate::server::acme::Acme;
use crate::server::routes::router;
use async_trait::async_trait;
#[cfg(test)]
use mockall::mock;
use std::sync::Arc;

pub fn new(config: Arc<Config>) -> impl WebServer {
    WebServerImpl {
        acme: Arc::new(Acme::new(&config)),
        config: config.clone(),
        web_channel_handle: WebEventChannelHandle::new(),
    }
}

#[async_trait]
pub trait WebServer: 'static + Clone + Send + Sync {
    async fn serve(&self, node_handles: &NodeHandles) -> Result<(), String>;
    fn get_flow_delegate_factory(&self, manager: &Manager) -> OauthFlowDelegateFactory;
    fn get_web_channel_handle(&self) -> &WebEventChannelHandle;
}

#[cfg(test)]
mock! {
    pub WebServer {}

    impl Clone for WebServer {
        fn clone(&self) -> Self;
    }

    #[async_trait]
    impl WebServer for WebServer {
        async fn serve(&self, node_handles: &NodeHandles) -> Result<(), String>;
        fn get_flow_delegate_factory(&self, manager: &Manager) -> OauthFlowDelegateFactory;
        fn get_web_channel_handle(&self) -> &WebEventChannelHandle;
    }
}

pub struct WebServerImpl {
    acme: Arc<Acme>,
    config: Arc<Config>,
    web_channel_handle: WebEventChannelHandle,
}

impl Clone for WebServerImpl {
    fn clone(&self) -> Self {
        Self {
            acme: self.acme.clone(),
            config: self.config.clone(),
            web_channel_handle: self.web_channel_handle.clone(),
        }
    }
}

#[async_trait]
impl WebServer for WebServerImpl {
    async fn serve(&self, node_handles: &NodeHandles) -> Result<(), String> {
        if self.config.domain_is_defined() {
            self.acme.ensure_certs().await?;
        }

        let routes = router(node_handles);
        let mut lifecycle_rx = node_handles.lifecycle_manager().readonly().get_receiver();
        let shutdown_binding = async move {
            loop {
                if lifecycle_rx.recv().await.is_stop() {
                    break;
                }
            }
        };
        let path_params = ([0, 0, 0, 0], self.config.port());
        let server = warp::serve(routes);

        if self.config.domain_is_defined() {
            let (addr, fut) = server
                .tls()
                .cert_path(self.acme.cert_path())
                .key_path(self.acme.key_path())
                .bind_with_graceful_shutdown(path_params, shutdown_binding);

            log::debug!("TLS Server listening on {}", addr);
            fut.await;
        } else {
            let (addr, fut) = server.bind_with_graceful_shutdown(path_params, shutdown_binding);

            log::debug!("Server listening on {}", addr);
            fut.await;
        };

        Ok(())
    }

    fn get_flow_delegate_factory(&self, manager: &Manager) -> OauthFlowDelegateFactory {
        OauthFlowDelegateFactory::new(
            manager,
            self.config.redirect_port(),
            &self.config.redirect_uri(),
            &self.web_channel_handle,
        )
    }

    fn get_web_channel_handle(&self) -> &WebEventChannelHandle {
        &self.web_channel_handle
    }
}
