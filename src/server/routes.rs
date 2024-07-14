use crate::server::auth::auth_validation;
use crate::server::page::login;
use crate::server::page::login::login;
use warp::{reply, Filter, Rejection};

pub fn router() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    root()
        .or(login())
        .recover(login::handlers::handle_rejection)
        .with(warp::log("api"))
}

fn root() -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    warp::path::end()
        .and(auth_validation())
        .map(move || reply::html(include_str!("../../resources/html/index.html")))
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
            use crate::core::root_password::test::with_test_root_password_scope;
            use crate::server::auth::gen_token_for_path;
            use crate::server::page::login::{LOGIN_FAILED, LOGIN_PATH};
            use warp::http::header::COOKIE;

            #[tokio::test]
            async fn authorized_root_serves_page() {
                let filter = router();
                let token = gen_token_for_path("/");
                let res = request()
                    .method("GET")
                    .header(COOKIE, token.to_cookie_string())
                    .path("/")
                    .reply(&filter)
                    .await;

                assert_eq!(res.status(), StatusCode::OK);
                assert_eq!(res.body(), include_str!("../../resources/html/index.html"));
                assert_eq!(
                    res.headers().get("content-type").unwrap(),
                    "text/html; charset=utf-8"
                );
            }

            #[tokio::test]
            async fn missing_token_redirects_to_login() {
                let filter = router();
                let res = request().method("GET").path("/").reply(&filter).await;

                assert_eq!(res.status(), StatusCode::FOUND);
                assert_eq!(
                    res.headers().get("location").unwrap().to_str().unwrap(),
                    LOGIN_PATH
                );
            }

            #[tokio::test]
            async fn bad_token_redirects_to_login() {
                let filter = router();
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
                let filter = router();
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
                    crate::server::page::login::format_login_html(true)
                );
            }
        }
    }
}
