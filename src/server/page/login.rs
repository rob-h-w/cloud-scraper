use crate::core::root_password::check_root_password;
use handlebars::Handlebars;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use warp::http::header::LOCATION;
use warp::{reply, Filter};

const LOGIN: &str = "login";

static PAGE_TEMPLATE: Lazy<Handlebars> = Lazy::new(|| {
    let mut handlebars = Handlebars::new();
    handlebars
        .register_template_string(LOGIN, include_str!("../../../resources/html/login.html"))
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
        .and(warp::query::<LoginQuery>())
        .map(move |query: LoginQuery| reply::html(format_login_html(query.failed())))
        .or(warp::path(LOGIN)
            .and(warp::post())
            .and(warp::body::form())
            .and_then(handlers::login_post))
}

fn format_login_html(failed: bool) -> String {
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
        .render(LOGIN, &page_data)
        .expect("Could not render login template")
}

async fn root_password_is_good(map: HashMap<String, String>) -> bool {
    match map.get("password") {
        Some(password) => check_root_password(password).await.unwrap_or(false),
        None => false,
    }
}

mod handlers {
    use super::*;
    use warp::http::StatusCode;

    pub async fn login_post(
        form_map: HashMap<String, String>,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let reply = reply::html("");
        let reply = if root_password_is_good(form_map).await {
            reply::with_header(reply, LOCATION, "/")
        } else {
            reply::with_header(reply, LOCATION, "/login?failed=true")
        };

        Ok(reply::with_status(reply, StatusCode::FOUND))
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

        #[tokio::test]
        async fn incorrect_password_redirects_to_login() {
            let _scope = with_test_root_password_scope().await;
            let filter = login();
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
                format_login_html(true)
            );
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
