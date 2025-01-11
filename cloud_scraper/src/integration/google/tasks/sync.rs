use crate::domain::oauth2::Client;
use crate::integration::google::auth::Delegate;
use google_tasks1::{hyper_rustls, TasksHub};
use log::info;

pub(crate) async fn sync<T>(delegate: Delegate<T>)
where
    T: Client,
{
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build(
            hyper_rustls::HttpsConnectorBuilder::new()
                .with_native_roots()
                .unwrap()
                .https_or_http()
                .enable_http1()
                .build(),
        );
    let hub = TasksHub::new(client, delegate);
    let task_lists = hub.tasklists().list().doit().await;
    info!("task lists: {:?}", task_lists);
}
