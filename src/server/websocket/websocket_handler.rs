use crate::core::node_handles::NodeHandles;
use crate::server::websocket::result::ResultRejection;

#[cfg(test)]
use std::ops::Deref;

#[cfg(not(test))]
use crate::server::websocket::use_websocket::use_websocket;

#[cfg(test)]
use warp::ws::WebSocket;
use warp::ws::Ws;
use warp::Reply;

pub(crate) async fn handler(ws: Ws, handles: NodeHandles) -> ResultRejection<impl Reply> {
    Ok(ws.on_upgrade(move |socket| use_websocket(socket, handles.clone())))
}

#[cfg(test)]
async fn use_websocket(_socket: WebSocket, _handles: NodeHandles) {
    use tests::CALL_COUNT;
    let lock = CALL_COUNT.lock();
    *lock.deref().borrow_mut() += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;
    use parking_lot::ReentrantMutex;
    use std::cell::RefCell;
    use std::sync::Arc;

    lazy_static! {
        pub(super) static ref CALL_COUNT: Arc<ReentrantMutex<RefCell<u32>>> =
            Arc::new(ReentrantMutex::new(RefCell::new(0)));
    }

    mod handler {
        use super::*;
        use crate::core::node_handles::tests::get_test_node_handles;
        use std::ops::Deref;
        use warp::Filter;

        #[tokio::test]
        async fn calls_on_upgrade() {
            let lock = CALL_COUNT.lock();
            *lock.deref().borrow_mut() = 0;

            let route = warp::ws()
                .map(|ws: Ws| handler(ws, get_test_node_handles()))
                .and_then(|future| future);

            let _client = warp::test::ws()
                .path("/ws")
                .handshake(route)
                .await
                .expect("handshake");

            assert_eq!(*lock.deref().borrow(), 1);
        }
    }
}
