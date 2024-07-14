use crate::core::root_password::check_root_password;
use crate::server::auth::auth_validation;
use handlebars::Handlebars;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use warp::http::header::LOCATION;
use warp::{reply, Filter};

const LOGIN_TEMPLATE: &str = "login";
const LOGIN: &str = "login";
pub const LOGIN_PATH: &str = "/login";
pub const LOGIN_FAILED: &str = "/login?failed=true";

static PAGE_TEMPLATE: Lazy<Handlebars> = Lazy::new(|| {
    let mut handlebars = Handlebars::new();
    handlebars
        .register_template_string(
            LOGIN_TEMPLATE,
            include_str!(
                "../../../resources/html/login\
        .html"
            ),
        )
        .expect("Could not register login template");
    handlebars
});

#[derive(Debug, Deserialize, Serialize)]
struct LoginQuery {
    failed: Option<bool>,
}

impl LoginQuery {
    fn failed(&self) -> bool {
        self.failed.unwrap_or(false)
    }
}

pub fn login() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path(LOGIN)
        .and(warp::get())
        .and(auth_validation())
        .map(|| warp::redirect::found(warp::http::Uri::from_static("/")))
        .or(warp::path(LOGIN)
            .and(warp::get())
            .and(warp::query::<LoginQuery>())
            .map(move |query: LoginQuery| reply::html(format_login_html(query.failed()))))
        .or(warp::path(LOGIN)
            .and(warp::post())
            .and(warp::body::form())
            .and_then(handlers::check_root_password)
            .map(handlers::issue_token_and_redirect))
}

pub fn format_login_html(failed: bool) -> String {
    let mut page_data = HashMap::new();
    page_data.insert(
        "error_html",
        if failed {
            r"<p><b>Failed to login.</b></p>"
        } else {
            r""
        },
    );

    PAGE_TEMPLATE
        .render(LOGIN_TEMPLATE, &page_data)
        .expect("Could not render login template")
}

async fn root_password_is_good(map: HashMap<String, String>) -> bool {
    match map.get("password") {
        Some(password) => check_root_password(password).await.unwrap_or(false),
        None => false,
    }
}

pub mod handlers {
    use super::*;
    use crate::server::auth::{gen_token_for_path, Unauthorized};
    use warp::http::header::SET_COOKIE;
    use warp::http::StatusCode;
    use warp::reject::InvalidHeader;
    use warp::Rejection;

    pub async fn check_root_password(
        form_map: HashMap<String, String>,
    ) -> Result<impl warp::Reply, Rejection> {
        if root_password_is_good(form_map).await {
            log::info!("Successfully logged in.");
            Ok(reply::html(""))
        } else {
            log::warn!("Failed to login because of bad password.");
            Err(Unauthorized::rejection())
        }
    }

    pub fn issue_token_and_redirect(reply: impl warp::Reply + Sized) -> impl warp::Reply {
        let token = gen_token_for_path("/");
        reply::with_status(
            reply::with_header(
                reply::with_header(reply, SET_COOKIE, token.to_cookie_string()),
                LOCATION,
                "/",
            ),
            StatusCode::FOUND,
        )
    }

    pub async fn handle_rejection(rejection: Rejection) -> Result<impl warp::Reply, Rejection> {
        let mut redirection: Option<Box<dyn warp::Reply>> = None;

        if let Some(invalid_header) = rejection.find::<InvalidHeader>() {
            if invalid_header.name() == "cookie" {
                redirection = Some(Box::new(warp::redirect::found(
                    warp::http::Uri::from_static(LOGIN_PATH),
                )))
            }
        } else if let Some(_unauthorized) = rejection.find::<Unauthorized>() {
            redirection = Some(Box::new(warp::redirect::found(
                warp::http::Uri::from_static(LOGIN_FAILED),
            )))
        }

        if let Some(redirection) = redirection {
            Ok(redirection)
        } else {
            Err(rejection)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp::http::StatusCode;
    use warp::test::request;

    mod format_login {
        use super::*;

        #[test]
        fn failed() {
            let html = format_login_html(true);
            assert!(html.contains("<p><b>Failed to login.</b></p>"));
        }

        #[test]
        fn not_failed() {
            let html = format_login_html(false);
            assert!(!html.contains("<p><b>Failed to login.</b></p>"));
        }
    }

    mod get_login {
        use super::*;
        use crate::server::auth::gen_token_for_path;
        use warp::http::header::COOKIE;

        #[tokio::test]
        async fn shows_the_page() {
            let filter = login();
            let res = request().method("GET").path("/login").reply(&filter).await;

            assert_eq!(res.status(), StatusCode::OK);
            assert_eq!(
                String::from_utf8(res.body().to_vec()).unwrap(),
                format_login_html(false)
            );
            assert_eq!(
                res.headers().get("content-type").unwrap(),
                "text/html; charset=utf-8"
            );
        }

        #[tokio::test]
        async fn with_auth_cookie_redirects_to_root() {
            let token = gen_token_for_path("/");
            let filter = login();
            let res = request()
                .method("GET")
                .header(COOKIE, token.to_cookie_string())
                .path("/login")
                .reply(&filter)
                .await;

            assert_eq!(res.status(), StatusCode::FOUND);
            assert_eq!(
                res.headers().get("location").unwrap().to_str().unwrap(),
                "/"
            );
        }
    }

    mod post_login {
        use super::*;
        use crate::core::root_password::test::{with_test_root_password_scope, TEST_PASSWORD};

        #[tokio::test]
        async fn correct_password_redirects_to_root() {
            let _scope = with_test_root_password_scope().await;
            let filter = login();
            let res = request()
                .method("POST")
                .path("/login")
                .body(format!("password={}", TEST_PASSWORD))
                .reply(&filter)
                .await;

            assert_eq!(res.status(), StatusCode::FOUND);
            assert_eq!(res.headers().get("location").unwrap(), "/");
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

        #[tokio::test]
        async fn bad_password_fails() {
            let _scope = with_test_root_password_scope().await;

            let mut map = HashMap::new();
            map.insert("password".to_string(), "bad".to_string());
            assert!(!root_password_is_good(map).await);
        }

        #[tokio::test]
        async fn no_password_saved_fails() {
            let mut map = HashMap::new();
            map.insert("password".to_string(), "missing".to_string());
            assert!(!root_password_is_good(map).await);
        }

        #[tokio::test]
        async fn no_password_fails() {
            let _scope = with_test_root_password_scope().await;

            let map = HashMap::new();
            assert!(!root_password_is_good(map).await);
        }
    }
}
