mod acme;
pub mod auth;
mod page;
mod routes;
mod site_state;

use crate::domain::config::Config;
use crate::domain::node::{Lifecycle, LifecycleAware};
use crate::server::acme::{Acme, AcmeImpl};
use crate::server::routes::router;
use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use std::sync::Arc;
use tokio::sync::broadcast::Receiver;

pub fn new<ConfigType>(config: Arc<ConfigType>) -> impl WebServer
where
    ConfigType: Config,
{
    WebServerImpl {
        acme: AcmeImpl::new(config.clone()),
        config: config.clone(),
    }
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait WebServer: Send + Sync {
    async fn serve(&self, stop_rx: Receiver<Lifecycle>) -> Result<(), String>;
}

pub struct WebServerImpl<AcmeType, ConfigType>
where
    AcmeType: Acme,
    ConfigType: Config,
{
    acme: AcmeType,
    config: Arc<ConfigType>,
}

impl<AcmeType, ConfigType> WebServerImpl<AcmeType, ConfigType>
where
    AcmeType: Acme,
    ConfigType: Config,
{
}

#[async_trait]
impl<AcmeType, ConfigType> WebServer for WebServerImpl<AcmeType, ConfigType>
where
    AcmeType: Acme,
    ConfigType: Config,
{
    async fn serve(&self, mut lifecycle_rx: Receiver<Lifecycle>) -> Result<(), String> {
        if self.config.domain_is_defined() {
            self.acme.ensure_certs().await?;
        }

        let routes = router();
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
