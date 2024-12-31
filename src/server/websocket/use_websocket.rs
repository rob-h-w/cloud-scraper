use crate::core::node_handles::NodeHandles;
use crate::domain::node::InitReplier;
use crate::server::Event::Redirect;
use crate::server::{Event, WebEventChannelHandle};
use log::{debug, error, info};
use std::error::Error;
use std::future;
use std::pin::Pin;
use tokio::sync::broadcast::error::RecvError;
use tokio::task;
use tokio_stream::StreamExt;
use warp::ws::{Message, WebSocket};
use warp::Sink;

pub(crate) async fn use_websocket(web_socket: WebSocket, handles: NodeHandles) {
    debug!("websocket created {:?}", web_socket);
    let handles = handles.clone();

    let lifecycle_reader = handles.lifecycle_manager().readonly();
    let web_event_reader = handles.web_channel_handle().clone();

    let task = task::spawn(async move {
        task(web_event_reader, web_socket).await;
    });

    lifecycle_reader.abort_on_stop(&task).await;
}

fn create_redirect_json(url: &str) -> String {
    format!("{{\"type\":\"redirect_event\",\"url\":\"{}\"}}", url)
}

async fn task(web_event_reader: WebEventChannelHandle, mut web_socket: WebSocket) {
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
}

async fn break_on_web_event_result(
    result: Result<Event, RecvError>,
    web_socket: &mut WebSocket,
) -> bool {
    match result {
        Ok(event) => {
            return break_on_web_event(event, web_socket).await;
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

#[derive(Clone, Debug, PartialEq)]
enum BreakInstruction {
    None,
    Break,
    BreakParent,
    Continue,
}

macro_rules! dispatch_break_instruction_from {
    ($instruction:expr) => {
        match $instruction {
            BreakInstruction::None => {}
            BreakInstruction::Break => {
                break;
            }
            BreakInstruction::BreakParent => {
                return true;
            }
            BreakInstruction::Continue => {
                continue;
            }
        }
    };
}

trait BreakInstructionTranslator {
    fn none_or_break_parent(
        &self,
        none_message: &str,
        break_parent_message: &str,
    ) -> BreakInstruction;

    fn none_or_continue(&self, none_message: &str, continue_message: &str) -> BreakInstruction;
}

impl<E> BreakInstructionTranslator for Result<(), E>
where
    E: Error,
{
    fn none_or_break_parent(
        &self,
        none_message: &str,
        break_parent_message: &str,
    ) -> BreakInstruction {
        match self {
            Ok(_) => {
                debug!("{}", none_message);
                BreakInstruction::None
            }
            Err(e) => {
                error!("{}: {:?}", break_parent_message, e);
                BreakInstruction::BreakParent
            }
        }
    }

    fn none_or_continue(&self, none_message: &str, continue_message: &str) -> BreakInstruction {
        match self {
            Ok(_) => {
                debug!("{}", none_message);
                BreakInstruction::None
            }
            Err(e) => {
                error!("{}: {:?}", continue_message, e);
                BreakInstruction::Continue
            }
        }
    }
}

async fn poll_ready(mut pinned_socket: Pin<&mut &mut WebSocket>) -> BreakInstruction {
    future::poll_fn(|cx| pinned_socket.as_mut().poll_ready(cx))
        .await
        .none_or_break_parent("Websocket ready.", "Error polling websocket ready")
}

fn start_send(mut pinned_socket: Pin<&mut &mut WebSocket>, message: Message) -> BreakInstruction {
    pinned_socket
        .as_mut()
        .start_send(message)
        .none_or_break_parent(
            "Websocket redirect event send started.",
            "Error starting redirect event send",
        )
}

async fn poll_flush(mut pinned_socket: Pin<&mut &mut WebSocket>) -> BreakInstruction {
    future::poll_fn(|cx| pinned_socket.as_mut().poll_flush(cx))
        .await
        .none_or_continue("Websocket flushed.", "Error flushing websocket")
}

fn handle_redirect_confirmation<E>(option: Option<Result<Message, E>>) -> BreakInstruction
where
    E: Error,
{
    match option {
        Some(Ok(message)) => {
            if message.is_close() {
                error!("Websocket closed.");
            } else {
                debug!("Received redirect confirmation: {:?}", message);
            }
            BreakInstruction::Break
        }
        Some(Err(e)) => {
            error!("Error receiving redirect confirmation message: {:?}", e);
            BreakInstruction::Continue
        }
        None => {
            error!("No message received.");
            BreakInstruction::Continue
        }
    }
}

async fn receive_redirect_confirmation(
    mut pinned_socket: Pin<&mut &mut WebSocket>,
) -> BreakInstruction {
    handle_redirect_confirmation(pinned_socket.as_mut().next().await)
}

async fn break_on_web_event(event: Event, mut web_socket: &mut WebSocket) -> bool {
    if let Redirect(url, sender) = event {
        loop {
            info!("Redirecting to {:?}", url);
            let redirect_json = create_redirect_json(&url);
            debug!("redirect JSON {:?}", redirect_json);
            let message = Message::text(redirect_json);

            debug!("Polling websocket ready.");
            dispatch_break_instruction_from!(poll_ready(Pin::new(&mut web_socket)).await);

            debug!("Sending redirect event: {:?}", message);
            dispatch_break_instruction_from!(start_send(Pin::new(&mut web_socket), message));

            debug!("Polling websocket flush.");
            dispatch_break_instruction_from!(poll_flush(Pin::new(&mut web_socket)).await);

            debug!("receiving redirect confirmation");
            dispatch_break_instruction_from!(
                receive_redirect_confirmation(Pin::new(&mut web_socket)).await
            );
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
    use serde::Serializer;
    use std::fmt::{Debug, Display, Formatter};
    use warp::Filter;

    #[derive(Debug)]
    struct TestError {}

    impl Display for TestError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.serialize_str("TestError")
        }
    }

    impl Error for TestError {}

    mod use_websocket {
        use super::*;
        use crate::core::node_handles::tests::get_test_node_handles;

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

    mod BreakInstructionTranslator {
        use super::*;
        use crate::server::websocket::use_websocket::BreakInstructionTranslator;
        mod none_or_break_parent {
            use super::*;

            #[test]
            fn test_none_or_break_parent_ok() {
                let result: Result<(), TestError> = Ok(());
                let none_message = "None message";
                let break_parent_message = "Break parent message";
                let break_instruction =
                    result.none_or_break_parent(none_message, break_parent_message);
                assert_eq!(break_instruction, BreakInstruction::None);
            }

            #[test]
            fn test_none_or_break_parent_err() {
                let result: Result<(), TestError> = Err(TestError {});
                let none_message = "None message";
                let break_parent_message = "Break parent message";
                let break_instruction =
                    result.none_or_break_parent(none_message, break_parent_message);
                assert_eq!(break_instruction, BreakInstruction::BreakParent);
            }
        }
        mod none_or_continue {
            use super::*;

            #[test]
            fn test_none_or_continue_ok() {
                let result: Result<(), TestError> = Ok(());
                let none_message = "None message";
                let continue_message = "Continue message";
                let break_instruction = result.none_or_continue(none_message, continue_message);
                assert_eq!(break_instruction, BreakInstruction::None);
            }

            #[test]
            fn test_none_or_continue_err() {
                let result: Result<(), TestError> = Err(TestError {});
                let none_message = "None message";
                let continue_message = "Continue message";
                let break_instruction = result.none_or_continue(none_message, continue_message);
                assert_eq!(break_instruction, BreakInstruction::Continue);
            }
        }
    }

    mod dispatch_break_instruction_from {
        use super::*;

        fn test_dispatch_break_instruction_from(
            instruction: BreakInstruction,
            instruction_out: &mut Option<BreakInstruction>,
        ) -> bool {
            for i in 0..2 {
                if i == 1 {
                    *instruction_out = Some(BreakInstruction::Continue);
                    return false;
                }
                dispatch_break_instruction_from!(instruction);
                *instruction_out = Some(BreakInstruction::None);

                return false;
            }

            *instruction_out = Some(BreakInstruction::Break);
            false
        }

        #[test]
        fn test_dispatch_break_instruction_from_none() {
            let mut instruction = None;
            test_dispatch_break_instruction_from(BreakInstruction::None, &mut instruction);
            assert_eq!(instruction, Some(BreakInstruction::None));
        }

        #[test]
        fn test_dispatch_break_instruction_from_break() {
            let mut instruction = None;
            test_dispatch_break_instruction_from(BreakInstruction::Break, &mut instruction);
            assert_eq!(instruction, Some(BreakInstruction::Break));
        }

        #[test]
        fn test_dispatch_break_instruction_from_break_parent() {
            let mut instruction = None;
            assert!(test_dispatch_break_instruction_from(
                BreakInstruction::BreakParent,
                &mut instruction
            ));
        }

        #[test]
        fn test_dispatch_break_instruction_from_continue() {
            let mut instruction = None;
            test_dispatch_break_instruction_from(BreakInstruction::Continue, &mut instruction);
            assert_eq!(instruction, Some(BreakInstruction::Continue));
        }
    }

    mod handle_redirect_confirmation {
        use super::*;

        #[test]
        fn breaks_on_websocket_close() {
            let message = Some(Ok(Message::close()));
            let result = handle_redirect_confirmation::<TestError>(message);
            assert_eq!(result, BreakInstruction::Break);
        }

        #[test]
        fn breaks_on_redirect_confirmation() {
            let message = Some(Ok(Message::text("redirect")));
            let result = handle_redirect_confirmation::<TestError>(message);
            assert_eq!(result, BreakInstruction::Break);
        }

        #[test]
        fn continues_on_error() {
            let message = Some(Err(TestError {}));
            let result = handle_redirect_confirmation::<TestError>(message);
            assert_eq!(result, BreakInstruction::Continue);
        }

        #[test]
        fn continues_on_none() {
            let message = None;
            let result = handle_redirect_confirmation::<TestError>(message);
            assert_eq!(result, BreakInstruction::Continue);
        }
    }

    mod break_on_web_event_result {
        use super::*;
        use crate::domain::mpsc_handle::one_shot;
        use lazy_static::lazy_static;
        use std::sync::Arc;
        use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};
        use warp::Filter;

        lazy_static! {
            pub static ref PERMIT: Arc<Mutex<Option<OwnedSemaphorePermit>>> =
                Arc::new(Mutex::new(None));
            pub static ref RESULT: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
            pub static ref SEMAPHORE: Arc<Mutex<Arc<Semaphore>>> =
                Arc::new(Mutex::new(Arc::new(Semaphore::new(1))));
            pub static ref TEST_LOCK: Mutex<()> = Mutex::new(());
        }

        #[tokio::test]
        async fn breaks_on_redirect() {
            let _test_lock = TEST_LOCK.lock().await;
            {
                let mut lock = RESULT.lock().await;
                *lock = true;
                let lock = SEMAPHORE.lock().await;
                let mut permit_lock = PERMIT.lock().await;
                *permit_lock = Some(lock.clone().acquire_owned().await.unwrap());
            }

            async fn callback(mut web_socket: WebSocket) {
                task::spawn(async move {
                    let (sender, _receiver) = one_shot();

                    let mut lock = RESULT.lock().await;
                    *lock = break_on_web_event_result(
                        Ok(Redirect(
                            "http://localhost:41047".to_string(),
                            sender.clone(),
                        )),
                        &mut web_socket,
                    )
                    .await;

                    // Wait in another thread until the semaphore is released instead of awaiting.
                    let mut permit_lock = PERMIT.lock().await;
                    drop(permit_lock.take().unwrap());
                });
            }

            let route = warp::ws().map(|ws: warp::ws::Ws| ws.on_upgrade(callback));
            let mut web_socket = warp::test::ws().handshake(route).await.expect("handshake");
            web_socket.send(Message::text("")).await;

            let lock = SEMAPHORE.lock().await;
            let _ = lock.acquire().await.unwrap();

            let lock = RESULT.lock().await;
            assert!(*lock, "Expected break_on_web_event_result to return true.");
        }

        #[tokio::test]
        async fn breaks_on_closed() {
            let _test_lock = TEST_LOCK.lock().await;
            {
                let mut lock = RESULT.lock().await;
                *lock = false;
            }
            async fn callback(mut web_socket: WebSocket) {
                let mut lock = RESULT.lock().await;
                *lock = break_on_web_event_result(Err(RecvError::Closed), &mut web_socket).await;
            }

            let route = warp::ws().map(|ws: warp::ws::Ws| ws.on_upgrade(callback));
            let _web_socket = warp::test::ws().handshake(route).await.expect("handshake");

            let lock = RESULT.lock().await;
            assert!(*lock, "Expected break_on_web_event_result to return true.");
        }

        #[tokio::test]
        async fn does_not_break_on_lag() {
            let _test_lock = TEST_LOCK.lock().await;
            {
                let mut lock = RESULT.lock().await;
                *lock = true;
            }
            async fn callback(mut web_socket: WebSocket) {
                let mut lock = RESULT.lock().await;
                *lock = break_on_web_event_result(Err(RecvError::Lagged(1)), &mut web_socket).await;
            }

            let route = warp::ws().map(|ws: warp::ws::Ws| ws.on_upgrade(callback));
            let _web_socket = warp::test::ws().handshake(route).await.expect("handshake");

            let lock = RESULT.lock().await;
            assert!(
                !*lock,
                "Expected break_on_web_event_result to return false."
            );
        }
    }
}
