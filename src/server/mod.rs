use crate::domain::config::Config;
use async_trait::async_trait;
use std::future::Future;
use std::sync::Arc;
use warp::Filter;

pub(crate) fn new<ConfigType>(config: Arc<ConfigType>) -> impl WebServer
where
    ConfigType: Config,
{
    WebServerImpl::new(config)
}

#[cfg(test)]
use mockall::automock;
use tokio::sync::broadcast::error::TryRecvError;

#[async_trait]
#[cfg_attr(test, automock)]
pub(crate) trait WebServer: Send + Sync {
    async fn serve(&self, stop_rx: tokio::sync::broadcast::Receiver<bool>);
}

pub(crate) struct WebServerImpl<ConfigType>
where
    ConfigType: Config,
{
    config: Arc<ConfigType>,
}

impl<ConfigType> WebServerImpl<ConfigType>
where
    ConfigType: Config,
{
    pub(crate) fn new(config: Arc<ConfigType>) -> Self {
        Self {
            config: config.clone(),
        }
    }
}

#[async_trait]
impl<ConfigType> WebServer for WebServerImpl<ConfigType>
where
    ConfigType: Config,
{
    async fn serve(&self, mut stop_rx: tokio::sync::broadcast::Receiver<bool>) {
        let routes = router();
        let (addr, fut) = warp::serve(routes)
            // .tls()
            .bind_with_graceful_shutdown(([127, 0, 0, 1], self.config.port()), async move {
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
            });
        log::debug!("Server listening on {}", addr);
        fut.await;
    }
}

fn router() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("new").map(move || "This is a new HTTP endpoint stub!")
}

#[cfg(test)]
mod tests {
    use warp::http::StatusCode;
    use warp::test::request;

    use super::*;

    #[tokio::test]
    async fn test_filter() {
        let filter = router();
        let res = request().method("GET").path("/new").reply(&filter).await;

        assert_eq!(res.status(), StatusCode::OK);
    }
}
