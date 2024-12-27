use crate::core::node_handles::NodeHandles;
#[cfg(test)]
use crate::server::websocket::result::ResultRejection;
#[cfg(not(test))]
use crate::server::websocket::websocket_handler::handler;
#[cfg(test)]
use warp::ws::Ws;
use warp::{path, ws, Filter, Rejection, Reply};

pub fn websocket(
    handles: &NodeHandles,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let handles = handles.clone();
    path("ws")
        .and(ws())
        .map(move |ws| handler(ws, handles.clone()))
        .and_then(|future| future)
}

#[cfg(test)]
pub(crate) async fn handler(ws: Ws, handles: NodeHandles) -> ResultRejection<impl Reply> {
    use std::future;

    Ok(ws.on_upgrade(move |_| {
        unsafe {
            tests::HANDLES = Some(handles.clone());
        }
        future::ready(())
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) static mut HANDLES: Option<NodeHandles> = None;

    mod websocket {
        use super::*;
        use crate::core::node_handles::tests::get_test_node_handles;

        #[tokio::test]
        async fn calls_handler() {
            unsafe {
                HANDLES = None;
            }

            let route = websocket(&get_test_node_handles());

            let _client = warp::test::ws()
                .path("/ws")
                .handshake(route.clone())
                .await
                .expect("handshake");

            unsafe {
                assert!(HANDLES.is_some(), "handles not set");
            }
        }
    }
}
