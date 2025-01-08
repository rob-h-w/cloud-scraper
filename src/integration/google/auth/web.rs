use crate::core::node_handles::NodeHandles;
use crate::domain::node::Manager;
use crate::domain::oauth2::BasicClientImpl;
use crate::domain::oauth2::PersistableConfig;
use crate::integration::google::auth::ConfigQuery;
use crate::integration::google::Source;
use crate::server::auth::auth_validation;
use crate::server::errors::Rejectable;
use crate::server::javascript::WithRedirect;
use handlebars::Handlebars;
use lazy_static::lazy_static;
use log::{debug, error};
use std::collections::HashMap;
use warp::{path, reply, Filter, Rejection, Reply};

const CONFIG_TEMPLATE: &str = "config/google";
const ROOT_PATH: &str = "/";

lazy_static! {
    pub static ref PAGE_TEMPLATE: Handlebars<'static> = {
        let mut handlebars = Handlebars::new();
        handlebars
            .register_template_string(
                CONFIG_TEMPLATE,
                include_str!("../../../../resources/html/config/google.html"),
            )
            .expect("Could not register Google config template");
        handlebars
    };
}

pub fn config_google(
    handles: &NodeHandles,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let get_node_handles = handles.clone();
    let post_node_handles = handles.clone();
    warp::path("config")
        .and(warp::path("google"))
        .and(path::end())
        .and(warp::get())
        .and(auth_validation())
        .map(move || {
            let node_handles = get_node_handles.clone();
            format_response(node_handles)
        })
        .and_then(|future| future)
        .or(warp::path("config")
            .and(warp::path("google"))
            .and(path::end())
            .and(warp::post())
            .and(auth_validation())
            .and(warp::body::form())
            .map(move |form_map| {
                let node_handles = post_node_handles.clone();
                update_config(form_map, node_handles)
            })
            .and_then(|future| future))
}

async fn format_response(handles: NodeHandles) -> Result<impl Reply, Rejection> {
    let existing_config = Source::<BasicClientImpl>::get_auth_config().await.ok();
    Ok(reply::html(
        format_config_google_html(handles, &existing_config).await,
    ))
}

async fn format_config_google_html(handles: NodeHandles, config: &Option<ConfigQuery>) -> String {
    let page_data = if let Some(config) = config {
        config.to_page_data()
    } else {
        ConfigQuery::empty_page_data()
    };

    let page_data = page_data.with_redirect_script(&handles);

    PAGE_TEMPLATE
        .render(CONFIG_TEMPLATE, &page_data)
        .expect("Could not render Google config page.")
}

async fn update_config(
    form_map: HashMap<String, String>,
    handles: NodeHandles,
) -> Result<impl Reply, Rejection> {
    let config = ConfigQuery::from(&form_map);

    let path = Source::<BasicClientImpl>::config_path()
        .await
        .map_err(|e| e.into_rejection())?;
    match config.persist(&path).await {
        Ok(_) => {
            let mut sender: Manager = handles.lifecycle_manager().clone();
            match sender.send_read_config::<Source<BasicClientImpl>>() {
                Ok(_) => {
                    debug!("Google config update sent");
                    Ok(warp::redirect::found(warp::http::Uri::from_static(
                        ROOT_PATH,
                    )))
                }
                Err(e) => {
                    error!("Error while sending Google config update: {:?}", e);
                    Err(e.into_rejection())
                }
            }
        }
        Err(e) => Err(e.into_rejection()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integration::google::auth::config::ConfigQueryBuilder;
    use crate::server::auth::gen_token_for_path;
    use crate::test::tests::CleanableTestFile;
    use lazy_static::lazy_static;
    use std::sync::Mutex;
    use tokio::fs;
    use warp::http::header::COOKIE;
    use warp::http::StatusCode;
    use warp::test::request;

    lazy_static! {
        pub static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
    }

    async fn make_config_file_and_lock<'a>() -> CleanableTestFile<'a> {
        CleanableTestFile::new(
            TEST_MUTEX.lock().expect("Could not lock mutex."),
            Source::<BasicClientImpl>::config_path()
                .await
                .expect("Could not get config path.")
                .to_str()
                .unwrap()
                .to_string(),
            |path| async move {
                let config = test_config();
                let serialized = serde_yaml::to_string(&config).unwrap();
                fs::write(path, serialized).await
            },
        )
        .await
    }

    async fn reset() {
        let config_path = Source::<BasicClientImpl>::config_path().await.unwrap();
        let _ = fs::remove_file(&config_path).await;
    }

    fn test_config() -> ConfigQuery {
        ConfigQueryBuilder::default()
            .project_id("test_project_id".into())
            .client_id("test_client_id".into())
            .client_secret("test_client_secret".into())
            .auth_uri("https://test.auth.uri".into())
            .auth_provider_x509_cert_url("test_auth_provider_x509_cert_url".into())
            .token_uri("https://test.token.uri".into())
            .build()
            .expect("Could not build test config")
    }

    fn test_config_form_encoded() -> String {
        "project_id=test_project_id&\
        client_id=test_client_id&\
        client_secret=test_client_secret&\
        auth_uri=https://test.auth.uri&\
        auth_provider_x509_cert_url=test_auth_provider_x509_cert_url&\
        token_uri=https://test.token.uri"
            .to_string()
    }

    mod config_google {
        use super::*;
        use crate::core::node_handles::tests::get_test_node_handles;
        use tokio_test::{assert_ok, task};

        #[tokio::test]
        async fn with_none_returns_html_with_defaults() {
            let _lock = make_config_file_and_lock().await;

            reset().await;

            let token = gen_token_for_path("/");
            let node_handles = get_test_node_handles();
            let filter = config_google(&node_handles);
            let res = request()
                .method("GET")
                .header(COOKIE, token.to_cookie_string())
                .path("/config/google")
                .reply(&filter)
                .await;

            let expected = format_config_google_html(node_handles, &None).await;
            let actual = String::from_utf8(res.body().to_vec()).unwrap();

            assert_eq!(res.status(), StatusCode::OK);
            assert_eq!(
                res.headers().get("content-type").unwrap(),
                "text/html; charset=utf-8"
            );
            assert_eq!(actual, expected);

            assert!(actual.contains("name=\"project_id\"\n"));
            assert!(actual.contains("name=\"client_id\"\n"));
            assert!(actual.contains("name=\"client_secret\"\n"));
            assert!(actual.contains(
                "name=\"auth_uri\" value=\"https://accounts.google.com/o/oauth2/auth\"\n"
            ));
            assert!(actual
                .contains("name=\"auth_provider_x509_cert_url\" value=\"https://www.googleapis.com/oauth2/v1/certs\"\n"));
            assert!(actual
                .contains("name=\"token_uri\" value=\"https://oauth2.googleapis.com/token\"\n"));
        }

        #[tokio::test]
        async fn with_config_returns_html_with_config_values() {
            let _lock = make_config_file_and_lock().await;

            let config = test_config();
            let token = gen_token_for_path("/");
            let node_handles = get_test_node_handles();
            let filter = config_google(&node_handles);
            let res = request()
                .method("GET")
                .header(COOKIE, token.to_cookie_string())
                .path("/config/google")
                .reply(&filter)
                .await;

            let expected = format_config_google_html(node_handles, &Some(config)).await;
            let actual = String::from_utf8(res.body().to_vec()).unwrap();

            assert_eq!(res.status(), StatusCode::OK);
            assert_eq!(
                res.headers().get("content-type").unwrap(),
                "text/html; charset=utf-8"
            );
            assert_eq!(actual, expected);

            assert!(actual.contains("name=\"project_id\" value=\"test_project_id\"\n"));
            assert!(actual.contains("name=\"client_id\" value=\"test_client_id\"\n"));
            assert!(actual.contains("name=\"client_secret\" value=\"test_client_secret\"\n"));
            assert!(actual.contains("name=\"auth_uri\" value=\"https://test.auth.uri\"\n"));
            assert!(actual.contains(
                "name=\"auth_provider_x509_cert_url\" value=\"test_auth_provider_x509_cert_url\"\n"
            ));
            assert!(actual.contains("name=\"token_uri\" value=\"https://test.token.uri\"\n"));
        }

        #[tokio::test]
        async fn post_translates_into_event() {
            let _lock = make_config_file_and_lock().await;

            let token = gen_token_for_path("/");
            let node_handles = get_test_node_handles();
            let mut lifecycle_handle = node_handles.lifecycle_manager().readonly().get_receiver();
            let lifecycle_abort_handle = task::spawn(async move {
                assert_ok!(lifecycle_handle.recv().await);
            });
            let filter = config_google(&node_handles);
            let res = request()
                .method("POST")
                .header(COOKIE, token.to_cookie_string())
                .path("/config/google")
                .body(test_config_form_encoded())
                .reply(&filter)
                .await;

            println!("{:?}", res);

            assert_eq!(res.status(), StatusCode::FOUND);
            assert_eq!(
                res.headers().get("location").unwrap().to_str().unwrap(),
                ROOT_PATH
            );
            lifecycle_abort_handle.await;
        }

        mod to_application_secret {
            use super::*;
            use crate::domain::{Config, DomainConfig};

            #[test]
            fn returns_application_secret() {
                let config = test_config();
                let core_config = Config::with_all_properties(
                    Some(DomainConfig::new("https://localhost")),
                    None,
                    None,
                    None,
                );
                let application_secret = config.to_application_secret(&core_config);
                assert_eq!(application_secret.client_id(), "test_client_id");
                assert_eq!(application_secret.client_secret(), "test_client_secret");
                assert_eq!(application_secret.auth_uri(), "https://test.auth.uri");
                assert_eq!(
                    application_secret.auth_provider_x509_cert_url(),
                    &Some("test_auth_provider_x509_cert_url".to_string())
                );
                assert_eq!(application_secret.token_uri(), "https://test.token.uri");
                assert_eq!(
                    application_secret.redirect_uris(),
                    &vec!["https://localhost/auth/google"]
                );
                assert_eq!(
                    application_secret.project_id(),
                    &Some("test_project_id".to_string())
                );
                assert_eq!(application_secret.client_email(), &None);
                assert_eq!(application_secret.client_x509_cert_url(), &None);
            }

            #[test]
            fn preserves_the_url_port() {
                let config = test_config();
                let core_config = Config::with_all_properties(
                    Some(DomainConfig::new("https://the.domain:8081")),
                    None,
                    None,
                    None,
                );
                let application_secret = config.to_application_secret(&core_config);
                assert_eq!(
                    application_secret.redirect_uris(),
                    &vec!["https://the.domain:8081/auth/google"]
                );
            }
        }
    }
}
