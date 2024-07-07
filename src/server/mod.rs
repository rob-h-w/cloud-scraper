mod acme;
mod routes;
mod site_state;

use crate::domain::config::Config;
use async_trait::async_trait;
use std::sync::Arc;

use crate::server::acme::{Acme, AcmeImpl};
use crate::server::routes::router;
#[cfg(test)]
use mockall::automock;

pub(crate) fn new<ConfigType>(config: Arc<ConfigType>) -> impl WebServer
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
pub(crate) trait WebServer: Send + Sync {
    async fn serve(&self, stop_rx: tokio::sync::broadcast::Receiver<bool>) -> Result<(), String>;
}

pub(crate) struct WebServerImpl<AcmeType, ConfigType>
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
    async fn serve(
        &self,
        mut stop_rx: tokio::sync::broadcast::Receiver<bool>,
    ) -> Result<(), String> {
        if self.config.domain_is_defined() {
            self.acme.ensure_certs().await?;
        }

        let routes = router();
        let shutdown_binding = async move {
            if !stop_rx.is_empty() {
                match stop_rx.try_recv() {
                    Ok(_) => {}
                    Err(_) => {
                        log::error!("Failed to receive stop signal");
                    }
                }
                return;
            }

            stop_rx.recv().await.expect("Failed to listen for stop");
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
