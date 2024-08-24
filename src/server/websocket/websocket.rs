use crate::core::node_handles::NodeHandles;
use crate::domain::node::InitReplier;
use crate::server::Event::Redirect;
use log::{debug, error, info};
use std::future;
use std::pin::Pin;
use tokio::sync::broadcast::error::RecvError;
use tokio::task;
use tokio_stream::StreamExt;
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
                        let mut exit_parent: bool = false;
                        loop {
                            info!("Redirecting to {:?}", url);
                            let redirect_json = create_redirect_json(&url);
                            debug!("redirect JSON {:?}", redirect_json);
                            let message = Message::text(redirect_json);

                            debug!("Polling websocket ready.");
                            let mut pinned_socket = Pin::new(&mut web_socket);
                            match future::poll_fn(|cx| pinned_socket.as_mut().poll_ready(cx)).await
                            {
                                Ok(_) => {
                                    debug!("Websocket ready.");
                                }
                                Err(e) => {
                                    error!("Error polling websocket ready: {:?}", e);
                                    exit_parent = true;
                                    break;
                                }
                            }

                            debug!("Sending redirect event: {:?}", message);
                            let mut pinned_socket = Pin::new(&mut web_socket);
                            match pinned_socket.as_mut().start_send(message) {
                                Ok(_) => {
                                    debug!("Websocket redirect event send started.");
                                }
                                Err(e) => {
                                    error!("Error starting redirect event send: {:?}", e);
                                    exit_parent = true;
                                    break;
                                }
                            }

                            debug!("Polling websocket flush.");
                            match future::poll_fn(|cx| pinned_socket.as_mut().poll_flush(cx)).await
                            {
                                Ok(_) => {
                                    debug!("Websocket flushed.");
                                }
                                Err(e) => {
                                    error!("Error flushing websocket: {:?}", e);
                                    continue;
                                }
                            }

                            debug!("receiving redirect confirmation");
                            let confirmation = match pinned_socket.as_mut().next().await {
                                Some(Ok(message)) => message,
                                Some(Err(e)) => {
                                    error!(
                                        "Error receiving redirect confirmation message: {:?}",
                                        e
                                    );
                                    continue;
                                }
                                None => {
                                    error!("No message received.");
                                    continue;
                                }
                            };

                            debug!("Received redirect confirmation: {:?}", confirmation);
                            break;
                        }

                        if exit_parent {
                            break;
                        }

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

fn create_redirect_json(url: &str) -> String {
    format!("{{\"type\":\"redirect_event\",\"url\":\"{}\"}}", url)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod create_redirect_json {
        use super::*;

        #[test]
        fn test_create_redirect_json() {
            let url = "https://accounts.google.com/o/oauth2/auth?scope=https://www.googleapis.com/auth/docs%20https://www.googleapis.com/auth/tasks&access_type=offline&redirect_uri=http://localhost:41047&response_type=code&client_id=922221802637-lmirsvbbfeub3fr2osf1n6lin67rhumg.apps.googleusercontent.com";
            let expected = "{\"type\":\"redirect_event\",\"url\":\"https://accounts.google.com/o/oauth2/auth?scope=https://www.googleapis.com/auth/docs%20https://www.googleapis.com/auth/tasks&access_type=offline&redirect_uri=http://localhost:41047&response_type=code&client_id=922221802637-lmirsvbbfeub3fr2osf1n6lin67rhumg.apps.googleusercontent.com\"}";
            let result = create_redirect_json(url);
            assert_eq!(result, expected);
        }
    }
}
