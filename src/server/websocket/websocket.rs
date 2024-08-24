use crate::core::node_handles::NodeHandles;
use crate::domain::node::{InitReplier, Lifecycle};
use crate::server::Event::Redirect;
use log::{debug, error, info};
use std::future;
use std::pin::Pin;
use tokio::sync::broadcast::error::RecvError;
use tokio::task;
use warp::ws::{Message, WebSocket};
use warp::{path, ws, Filter, Rejection, Reply, Sink};

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
    let web_event_reader = handles.web_channel_handle().clone();

    let task = task::spawn(async move {
        loop {
            match web_event_reader.get_receiver().recv().await {
                Ok(event) => match event {
                    Redirect(url, sender) => {
                        // TODO - retry on timeout
                        let message = Message::text(format!(
                            "{{\"type\":\"redirect_event\",\"url\":\"{:?}\"}}",
                            url
                        ));

                        debug!("Polling websocket ready.");
                        let mut pinned_socket = Pin::new(&mut web_socket);
                        future::poll_fn(|cx| pinned_socket.as_mut().poll_ready(cx))
                            .await
                            .unwrap();

                        debug!("Sending redirect event: {:?}", message);
                        let mut pinned_socket = Pin::new(&mut web_socket);
                        pinned_socket.as_mut().start_send(message).unwrap();

                        debug!("Polling websocket flush.");
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
                        };
                        break;
                    }
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

    lifecycle_reader.abort_on_stop(&task);
}
