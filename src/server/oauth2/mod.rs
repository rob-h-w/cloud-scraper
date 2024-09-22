// http://localhost:8080/auth/google?state=Gkkc4vMVgVEbJu5Rx8zvKg&code=4/0AQlEd8w4wkBrhELT5zbqKSLw8_JRKgCAWJgCBzdzqa8cQ5qeW4d-nNLTdAXQAJkYb4Di3w&scope=https://www.googleapis.com/auth/tasks%20https://www.googleapis.com/auth/docs
macro_rules! oauth2_callback {
    ($handles: expr) => {
        oauth2_callback!(@remaining $handles, warp::path::end(), std::concat!("/"))
    };
    ($handles: expr, $path: tt) => {
        oauth2_callback!(@remaining $handles, warp::path!($path), std::concat!("/", $path))
    };
    ($handles: expr, $first:tt $(/ $tail:tt)*) => {
        oauth2_callback!(@remaining $handles, warp::path!($first $(/ $tail)*), std::concat!("/", $first, $("/", $tail,)*))
    };
    (@remaining $handles:expr, $filter:expr, $path:expr) => {
        {
            use crate::server::Code;
            use crate::server::oauth2::send_code_and_redirect;

            use warp::Filter;

            let handles = $handles.clone();

            $filter
                .and(warp::get())
                .and(warp::query::<Code>())
                .map(move |code: Code| {
                    send_code_and_redirect(code, &handles, $path)
                })
        }
    };
}

pub(crate) use oauth2_callback;

use crate::core::node_handles::NodeHandles;
use crate::server::Code;
use crate::server::Event::Oauth2Code;
use warp::Reply;

pub(crate) fn send_code_and_redirect(code: Code, handles: &NodeHandles, path: &str) -> impl Reply {
    let handles = handles.clone();
    let mut web_channel_handle = handles.web_channel_handle().clone();
    if web_channel_handle
        .send(Oauth2Code(code, path.to_string()))
        .is_err()
    {
        log::error!("Failed to send oauth2 code to web channel");
    }

    warp::redirect::found(warp::http::Uri::from_static("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp::http::StatusCode;
    use warp::test::request;

    mod oauth2_callback {
        use super::*;
        use crate::core::node_handles::test::get_test_node_handles;
        use warp::{Filter, Rejection};

        fn expected_type(_: impl Filter<Extract = impl Reply, Error = Rejection> + Clone) {}

        #[test]
        fn returns_the_right_type() {
            let node_handles = get_test_node_handles();
            expected_type(oauth2_callback!(&node_handles));
            expected_type(oauth2_callback!(&node_handles, "oauth2"));
            expected_type(oauth2_callback!(&node_handles, "oauth2" / "google"));
        }

        #[tokio::test]
        async fn callback_path_2_elements() {
            let node_handles = get_test_node_handles();
            let filter = oauth2_callback!(&node_handles, "oauth2" / "google");
            let res = request()
                .method("GET")
                .path("/oauth2/google?code=123&state=abc")
                .reply(&filter)
                .await;

            assert_eq!(res.status(), StatusCode::FOUND);
            assert_eq!(res.headers().get("location").unwrap(), "/");
        }
    }
}
