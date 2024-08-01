mod acme;
pub mod auth;
pub mod errors;
mod page;
mod routes;
mod site_state;

use crate::core::node_handles::NodeHandles;
use crate::domain::config::Config;
use crate::domain::node::LifecycleAware;
use crate::server::acme::{Acme, AcmeImpl};
use crate::server::routes::router;
use async_trait::async_trait;
#[cfg(test)]
use mockall::mock;
use std::sync::Arc;

pub fn new<ConfigType>(config: Arc<ConfigType>) -> impl WebServer
where
    ConfigType: Config,
{
    WebServerImpl {
        acme: Arc::new(AcmeImpl::new(config.clone())),
        config: config.clone(),
    }
}

#[async_trait]
pub trait WebServer: 'static + Clone + Send + Sync {
    async fn serve(&self, node_handles: &NodeHandles) -> Result<(), String>;
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
    }
}

pub struct WebServerImpl<AcmeType, ConfigType>
where
    AcmeType: Acme,
    ConfigType: Config,
{
    acme: Arc<AcmeType>,
    config: Arc<ConfigType>,
}

impl<AcmeType, ConfigType> WebServerImpl<AcmeType, ConfigType>
where
    AcmeType: Acme,
    ConfigType: Config,
{
}

impl<AcmeType, ConfigType> Clone for WebServerImpl<AcmeType, ConfigType>
where
    AcmeType: Acme,
    ConfigType: Config,
{
    fn clone(&self) -> Self {
        Self {
            acme: self.acme.clone(),
            config: self.config.clone(),
        }
    }
}

#[async_trait]
impl<AcmeType, ConfigType> WebServer for WebServerImpl<AcmeType, ConfigType>
where
    AcmeType: Acme,
    ConfigType: Config,
{
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
}
