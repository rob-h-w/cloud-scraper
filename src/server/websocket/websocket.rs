use log::debug;
use std::future;
use std::pin::Pin;
use warp::ws::{Message, WebSocket};
use warp::{path, ws, Filter, Rejection, Reply, Sink};

type Result<T> = std::result::Result<T, Rejection>;

pub fn websocket() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path("ws").and(ws()).and_then(handler)
}

async fn handler(ws: ws::Ws) -> Result<impl Reply> {
    Ok(ws.on_upgrade(move |socket| use_websocket(socket)))
}

async fn use_websocket(mut web_socket: WebSocket) {
    debug!("websocket created {:?}", web_socket);
    let message = Message::text("Hello from the server");
    let mut pinned_socket = Pin::new(&mut web_socket);
    future::poll_fn(|cx| pinned_socket.as_mut().poll_ready(cx))
        .await
        .unwrap();
    let mut pinned_socket = Pin::new(&mut web_socket);
    pinned_socket.as_mut().start_send(message).unwrap();

    future::poll_fn(|cx| pinned_socket.as_mut().poll_flush(cx))
        .await
        .unwrap();
}
