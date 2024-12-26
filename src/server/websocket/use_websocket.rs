use crate::core::node_handles::NodeHandles;
use crate::domain::node::InitReplier;
use crate::server::Event;
use crate::server::Event::Redirect;
use log::{debug, error, info};
use std::future;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::broadcast::error::RecvError;
use tokio::task;
use tokio_stream::StreamExt;
use warp::ws::{Message, WebSocket};
use warp::Sink;

pub(crate) async fn use_websocket(mut web_socket: WebSocket, handles: NodeHandles) {
    debug!("websocket created {:?}", web_socket);
    let handles = handles.clone();

    let lifecycle_reader = handles.lifecycle_manager().readonly();
    let web_event_reader = handles.web_channel_handle().clone();

    let task = task::spawn(async move {
        loop {
            if break_on_web_event_result(
                web_event_reader.get_receiver().recv().await,
                &mut web_socket,
            )
            .await
            {
                break;
            }
        }
    });

    lifecycle_reader.abort_on_stop(&task).await;
}

fn create_redirect_json(url: &str) -> String {
    format!("{{\"type\":\"redirect_event\",\"url\":\"{}\"}}", url)
}

async fn break_on_web_event_result(
    result: Result<Event, RecvError>,
    mut web_socket: &mut WebSocket,
) -> bool {
    match result {
        Ok(event) => {
            return break_on_web_event(event, &mut web_socket).await;
        }
        Err(e) => match e {
            RecvError::Closed => {
                return true;
            }
            RecvError::Lagged(amount) => {
                info!("Websocket lagged by {} messages.", amount);
            }
        },
    }

    false
}

async fn break_on_web_event(event: Event, mut web_socket: &mut WebSocket) -> bool {
    if let Redirect(url, sender) = event {
        let mut exit_parent: bool = false;
        loop {
            info!("Redirecting to {:?}", url);
            let redirect_json = create_redirect_json(&url);
            debug!("redirect JSON {:?}", redirect_json);
            let message = Message::text(redirect_json);

            debug!("Polling websocket ready.");
            let mut pinned_socket = Pin::new(&mut web_socket);
            match future::poll_fn(|cx| pinned_socket.as_mut().poll_ready(cx)).await {
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
            match future::poll_fn(|cx| pinned_socket.as_mut().poll_flush(cx)).await {
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
                    error!("Error receiving redirect confirmation message: {:?}", e);
                    continue;
                }
                None => {
                    error!("No message received.");
                    continue;
                }
            };

            if confirmation.is_close() {
                error!("Websocket closed.");
                break;
            }

            debug!("Received redirect confirmation: {:?}", confirmation);
            break;
        }

        if exit_parent {
            return true;
        }

        match sender.send(url).await {
            Ok(_) => {
                debug!("Redirect websocket event sent.");
            }
            Err(_) => {
                error!("Redirect websocket event failed to send.");
            }
        };

        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod use_websocket {
        use super::*;
        use crate::core::node_handles::tests::get_test_node_handles;
        use warp::Filter;

        #[tokio::test]
        async fn exits_when_web_event_reader_is_closed() {
            let handles = get_test_node_handles();
            let route = warp::ws().map(move |ws: warp::ws::Ws| {
                let handles = handles.clone();
                ws.on_upgrade(move |socket| use_websocket(socket, handles))
            });
            let _client = warp::test::ws()
                .handshake(route.clone())
                .await
                .expect("handshake");
        }
    }

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

    mod break_on_web_event_result {
        use super::*;
        use crate::domain::mpsc_handle::one_shot;
        use warp::Filter;

        #[tokio::test]
        async fn breaks_on_redirect() {
            async fn callback(mut web_socket: WebSocket) {
                let (sender, _receiver) = one_shot();

                let result = break_on_web_event_result(
                    Ok(Redirect(
                        "http://localhost:41047".to_string(),
                        sender.clone(),
                    )),
                    &mut web_socket,
                )
                .await;

                assert!(result, "Expected true.");
            }

            let route = warp::ws().map(|ws: warp::ws::Ws| ws.on_upgrade(callback));
            let _web_socket = warp::test::ws().handshake(route).await.expect("handshake");
        }

        #[tokio::test]
        async fn breaks_on_closed() {
            async fn callback(mut web_socket: WebSocket) {
                let result =
                    break_on_web_event_result(Err(RecvError::Closed), &mut web_socket).await;

                assert!(result, "Expected true.");
            }

            let route = warp::ws().map(|ws: warp::ws::Ws| ws.on_upgrade(callback));
            let _web_socket = warp::test::ws().handshake(route).await.expect("handshake");
        }

        #[tokio::test]
        async fn does_not_break_on_lag() {
            async fn callback(mut web_socket: WebSocket) {
                let result =
                    break_on_web_event_result(Err(RecvError::Lagged(1)), &mut web_socket).await;

                assert!(!result, "Expected false.");
            }

            let route = warp::ws().map(|ws: warp::ws::Ws| ws.on_upgrade(callback));
            let _web_socket = warp::test::ws().handshake(route).await.expect("handshake");
        }
    }
}
