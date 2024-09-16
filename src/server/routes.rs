use crate::core::node_handles::NodeHandles;
use crate::integration::google::auth::web::config_google;
use crate::server::oauth2::oauth2_callback;
use crate::server::page::{handlers, login};
use crate::server::root::root;
use crate::server::websocket::websocket;
use warp::{Filter, Rejection};

pub fn router(
    handles: &NodeHandles,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    root(handles)
        .or(login())
        .or(config_google(handles))
        .or(websocket(handles))
        .or(oauth2_callback!(handles, "auth" / "google"))
        .recover(handlers::handle_rejection)
        .with(warp::log("api"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp::http::StatusCode;
    use warp::test::request;

    mod router {
        use super::*;

        mod auth {
            use super::*;
            use crate::core::node_handles::test::get_test_node_handles;
            use crate::core::root_password::test::with_test_root_password_scope;
            use crate::server::auth::gen_token_for_path;
            use crate::server::format_root_html;
            use crate::server::page::{LOGIN_FAILED, LOGIN_PATH};
            use warp::http::header::COOKIE;

            #[tokio::test]
            async fn authorized_root_serves_page() {
                let node_handles = get_test_node_handles();
                let filter = router(&node_handles);
                let token = gen_token_for_path("/");
                let res = request()
                    .method("GET")
                    .header(COOKIE, token.to_cookie_string())
                    .path("/")
                    .reply(&filter)
                    .await;

                let expected = format_root_html(&node_handles);
                assert_eq!(res.status(), StatusCode::OK);
                assert_eq!(res.body(), expected.as_bytes());
                assert_eq!(
                    res.headers().get("content-type").unwrap(),
                    "text/html; charset=utf-8"
                );
            }

            #[tokio::test]
            async fn missing_token_redirects_to_login() {
                let node_handles = get_test_node_handles();
                let filter = router(&node_handles);
                let res = request().method("GET").path("/").reply(&filter).await;

                assert_eq!(res.status(), StatusCode::FOUND);
                assert_eq!(
                    res.headers().get("location").unwrap().to_str().unwrap(),
                    LOGIN_PATH
                );
            }

            #[tokio::test]
            async fn bad_token_redirects_to_login() {
                let node_handles = get_test_node_handles();
                let filter = router(&node_handles);
                let res = request()
                    .method("GET")
                    .header(COOKIE, "token=bad_token")
                    .path("/")
                    .reply(&filter)
                    .await;

                assert_eq!(res.status(), StatusCode::FOUND);
                assert_eq!(
                    res.headers().get("location").unwrap().to_str().unwrap(),
                    LOGIN_FAILED
                );
            }

            #[tokio::test]
            async fn incorrect_password_redirects_to_login() {
                let _scope = with_test_root_password_scope().await;
                let node_handles = get_test_node_handles();
                let filter = router(&node_handles);
                let res = request()
                    .method("POST")
                    .path("/login")
                    .body("password=wrong")
                    .reply(&filter)
                    .await;

                assert_eq!(res.status(), StatusCode::FOUND);
                let location = res.headers().get("location").unwrap().to_str().unwrap();
                assert_eq!(location, "/login?failed=true");

                let res = request().method("GET").path(location).reply(&filter).await;

                assert_eq!(res.status(), StatusCode::OK);
                assert_eq!(
                    String::from_utf8(res.body().to_vec()).unwrap(),
                    crate::server::page::format_login_html(true)
                );
            }
        }
    }
}
