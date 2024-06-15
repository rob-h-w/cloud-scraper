use crate::domain::config::Config;
use async_trait::async_trait;
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

#[async_trait]
#[cfg_attr(test, automock)]
pub(crate) trait WebServer: Send + Sync {
    async fn serve(&self);
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
    async fn serve(&self) {
        let routes = router();
        warp::serve(routes)
            .run(([127, 0, 0, 1], self.config.port()))
            .await;
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
