use crate::core::node_handles::NodeHandles;
use crate::domain::node::{InitReplier, Lifecycle};
use log::{debug, error, info};
use std::future;
use std::pin::Pin;
use tokio::sync::broadcast::error::RecvError;
use tokio::task;
use warp::ws::{Message, WebSocket};
use warp::{path, ws, Filter, Rejection, Reply, Sink};
use Lifecycle::{Redirect, Stop};

type Result<T> = std::result::Result<T, Rejection>;

pub fn websocket(
    handles: &NodeHandles,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let handles = handles.clone();
    path("ws")
        .and(ws())
        .map(move |ws| {
            let handles = handles.clone();
            handler(ws, handles)
        })
        .and_then(|future| future)
}

async fn handler(ws: ws::Ws, handles: NodeHandles) -> Result<impl Reply> {
    Ok(ws.on_upgrade(move |socket| {
        let handles = handles.clone();
        use_websocket(socket, handles)
    }))
}

async fn use_websocket(mut web_socket: WebSocket, handles: NodeHandles) {
    debug!("websocket created {:?}", web_socket);
    let handles = handles.clone();

    let lifecycle_reader = handles.lifecycle_manager().readonly();

    task::spawn(async move {
        loop {
            match lifecycle_reader.get_receiver().recv().await {
                Ok(event) => match event {
                    Redirect(url, sender) => {
                        let message = Message::text(format!(
                            "{{\"type\":\"redirect_event\",\"url\":\"{:?}\"}}",
                            url
                        ));
                        let mut pinned_socket = Pin::new(&mut web_socket);
                        future::poll_fn(|cx| pinned_socket.as_mut().poll_ready(cx))
                            .await
                            .unwrap();
                        let mut pinned_socket = Pin::new(&mut web_socket);
                        pinned_socket.as_mut().start_send(message).unwrap();

                        future::poll_fn(|cx| pinned_socket.as_mut().poll_flush(cx))
                            .await
                            .unwrap();

                        match sender.send(url).await {
                            Ok(_) => {
                                debug!("Redirect websocket event sent.");
                            }
                            Err(_) => {
                                error!("Redirect websocket event failed to send.");
                            }
                        }
                    }
                    Stop => {
                        break;
                    }
                    _ => {}
                },
                Err(e) => match e {
                    RecvError::Closed => {
                        break;
                    }
                    RecvError::Lagged(amount) => {
                        info!("Websocket lagged by {} messages.", amount);
                    }
                },
            }
        }
    });
}
