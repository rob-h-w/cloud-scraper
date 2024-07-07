use crate::core::root_password::check_root_password;
use std::collections::HashMap;
use warp::http::header::LOCATION;
use warp::{reply, Filter};

pub fn router() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    root().or(login())
}

fn root() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path::end().map(move || reply::html(include_str!("../../resources/html/index.html")))
}

fn login() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    const LOGIN: &str = "login";
    warp::path(LOGIN)
        .and(warp::get())
        .map(move || reply::html(include_str!("../../resources/html/login.html")))
        .or(warp::path(LOGIN)
            .and(warp::post())
            .and(warp::body::form())
            .and_then(handlers::login_post)
            .with(warp::reply::with::header(LOCATION.as_str(), "/")))
}

mod handlers {
    use super::*;
    use crate::server::routes::root_password_is_good;
    use warp::http::StatusCode;

    pub async fn login_post(
        form_map: HashMap<String, String>,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        if root_password_is_good(form_map).await {
            Ok(reply::with_status(
                reply::html(""),
                StatusCode::TEMPORARY_REDIRECT,
            ))
        } else {
            Ok(reply::with_status(reply::html(""), StatusCode::FORBIDDEN))
        }
    }
}

async fn root_password_is_good(map: HashMap<String, String>) -> bool {
    match map.get("password") {
        Some(password) => check_root_password(password).await.unwrap_or(false),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp::http::StatusCode;
    use warp::test::request;

    #[tokio::test]
    async fn test_filter() {
        let filter = router();
        let res = request().method("GET").path("/").reply(&filter).await;

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.body(), include_str!("../../resources/html/index.html"));
        assert_eq!(
            res.headers().get("content-type").unwrap(),
            "text/html; charset=utf-8"
        );
    }

    mod root {
        use super::*;

        #[tokio::test]
        async fn shows_the_page() {
            let filter = root();
            let res = request().method("GET").path("/").reply(&filter).await;

            assert_eq!(res.status(), StatusCode::OK);
            assert_eq!(res.body(), include_str!("../../resources/html/index.html"));
            assert_eq!(
                res.headers().get("content-type").unwrap(),
                "text/html; charset=utf-8"
            );
        }
    }

    mod get_login {
        use super::*;

        #[tokio::test]
        async fn shows_the_page() {
            let filter = login();
            let res = request().method("GET").path("/login").reply(&filter).await;

            assert_eq!(res.status(), StatusCode::OK);
            assert_eq!(res.body(), include_str!("../../resources/html/login.html"));
            assert_eq!(
                res.headers().get("content-type").unwrap(),
                "text/html; charset=utf-8"
            );
        }
    }

    mod post_login {
        use super::*;
        use crate::core::root_password::test::{with_test_root_password_scope, TEST_PASSWORD};

        #[tokio::test]
        async fn correct_password_redirects() {
            let _scope = with_test_root_password_scope().await;
            let filter = login();
            let res = request()
                .method("POST")
                .path("/login")
                .body(format!("password={}", TEST_PASSWORD))
                .reply(&filter)
                .await;

            assert_eq!(res.status(), StatusCode::TEMPORARY_REDIRECT);
            assert_eq!(res.headers().get("location").unwrap(), "/");
        }

        #[tokio::test]
        async fn incorrect_password_forbids() {
            let _scope = with_test_root_password_scope().await;
            let filter = login();
            let res = request()
                .method("POST")
                .path("/login")
                .body("password=wrong")
                .reply(&filter)
                .await;

            assert_eq!(res.status(), StatusCode::FORBIDDEN);
        }
    }

    mod root_password_is_good {
        use super::*;
        use crate::core::root_password::test::{with_test_root_password_scope, TEST_PASSWORD};

        #[tokio::test]
        async fn success() {
            let _scope = with_test_root_password_scope().await;

            let mut map = HashMap::new();
            map.insert("password".to_string(), TEST_PASSWORD.to_string());
            assert!(root_password_is_good(map).await);
        }
    }
}
