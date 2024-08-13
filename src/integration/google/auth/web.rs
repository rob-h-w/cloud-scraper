use crate::core::module::State;
use crate::core::node_handles::NodeHandles;
#[cfg(not(test))]
use crate::domain::config::Config;
#[cfg(test)]
use crate::domain::config::{Config, DomainConfig};
use crate::domain::module_state::ModuleState;
use crate::domain::node::Manager;
use crate::integration::google::Source;
use crate::server::auth::auth_validation;
use crate::server::errors::Rejectable;
use crate::static_init::error::{Error, IoErrorExt, SerdeErrorExt};
use handlebars::Handlebars;
use log::{debug, error};
use once_cell::sync::Lazy;
use paste::paste;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use tokio::fs;
use warp::{reply, Filter, Rejection, Reply};
use yup_oauth2::ApplicationSecret;

const CONFIG_TEMPLATE: &str = "config/google";
const ROOT_PATH: &str = "/";

static PAGE_TEMPLATE: Lazy<Handlebars> = Lazy::new(|| {
    let mut handlebars = Handlebars::new();
    handlebars
        .register_template_string(
            CONFIG_TEMPLATE,
            include_str!("../../../../resources/html/config/google.html"),
        )
        .expect("Could not register Google config template");
    handlebars
});

macro_rules! make_config_query {
    ($struct:ident, { $($e:ident),* }, { $($d:ident, $v:literal),* }) => {
        paste! {
            #[derive(Debug, Deserialize, Serialize)]
            pub struct $struct {
                $(
                    $e: String,
                )*
                $(
                    $d: String,
                )*
            }

            impl $struct {
                fn empty_page_data() -> HashMap<&'static str, String> {
                    let mut page_data = HashMap::new();
                    $(
                        page_data.insert(stringify!($e), Self::format_empty(stringify!($e)));
                    )*
                    $(
                        page_data.insert(stringify!($d), Self::format(stringify!($d), $v));
                    )*
                    page_data
                }

                pub fn new(map: &HashMap<String, String>) -> Self {
                    Self {
                        $(
                            $e: map.get(stringify!($e)).unwrap_or(&String::new())
                            .clone(),
                        )*
                        $(
                            $d: map.get(stringify!($d)).unwrap_or
                            (&String::from($v)).clone(),
                        )*
                    }
                }

                fn format(name: &str, value: &str) -> String {
                    format!("name=\"{}\" value=\"{}\"", name,  value)
                }

                fn format_empty(name: &str) -> String {
                    format!("name=\"{}\"", name)
                }

                fn to_page_data(&self) -> HashMap<&'static str, String> {
                    let mut page_data = HashMap::new();
                    $(
                        page_data.insert(stringify!($e), Self::format(stringify!($e), &self.$e));
                    )*
                    $(
                        page_data.insert(stringify!($d), Self::format(stringify!($d), &self.$d));
                    )*
                    page_data
                }

                $(
                    pub fn $e(&self) -> String {
                        self.$e.clone()
                    }
                )*
                $(
                    pub fn $d(&self) -> String {
                        self.$d.clone()
                    }
                )*
            }
        }
    }
}
make_config_query!(
ConfigQuery,
{ project_id, client_id, client_secret },
{
    auth_uri, "https://accounts.google.com/o/oauth2/auth",
    auth_provider_x509_cert_url, "https://www.googleapis.com/oauth2/v1/certs",
    token_uri, "https://oauth2.googleapis.com/token"
});

impl ConfigQuery {
    pub fn to_application_secret(&self, config: &Config) -> ApplicationSecret {
        ApplicationSecret {
            client_id: self.client_id(),
            client_secret: self.client_secret(),
            auth_uri: self.auth_uri(),
            auth_provider_x509_cert_url: Some(self.auth_provider_x509_cert_url()),
            token_uri: self.token_uri(),
            redirect_uris: Self::make_redirect_uris(config),
            project_id: Some(self.project_id()),
            client_email: None,
            client_x509_cert_url: None,
        }
    }

    fn make_redirect_uris(config: &Config) -> Vec<String> {
        let scheme = if config.uses_tls() { "https" } else { "http" };
        let first_uri_element = if let Some(domain_config) = config.domain_config() {
            format!("{}://{}", scheme, domain_config.domain_name())
        } else {
            format!("{}://localhost", scheme)
        };
        let redirect_uri = format!(
            "{}:{:?}/sessions/auth/google",
            first_uri_element,
            config.port()
        );
        vec![redirect_uri]
    }
}

pub fn config_google(
    handles: &NodeHandles,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let node_handles = handles.clone();
    warp::path("config")
        .and(warp::path("google"))
        .and(warp::get())
        .and(auth_validation())
        .and_then(format_response)
        .or(warp::path("config")
            .and(warp::path("google"))
            .and(warp::post())
            .and(auth_validation())
            .and(warp::body::form())
            .map(move |form_map| {
                let node_handles = node_handles.clone();
                update_config(form_map, node_handles)
            })
            .and_then(|future| future))
}

async fn format_response() -> Result<impl Reply, Rejection> {
    let existing_config = get_config().await;
    Ok(reply::html(
        format_config_google_html(&existing_config).await,
    ))
}

async fn format_config_google_html(config: &Option<ConfigQuery>) -> String {
    let page_data = if let Some(config) = config {
        config.to_page_data()
    } else {
        ConfigQuery::empty_page_data()
    };

    PAGE_TEMPLATE
        .render(CONFIG_TEMPLATE, &page_data)
        .expect("Could not render Google config page.")
}

async fn update_config(
    form_map: HashMap<String, String>,
    handles: NodeHandles,
) -> Result<impl Reply, Rejection> {
    let config = ConfigQuery::new(&form_map);

    match put_config(&config).await {
        Ok(_) => {
            let mut sender: Manager = handles.lifecycle_manager().clone();
            match sender.send_read_config::<Source>() {
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

async fn config_path() -> Result<PathBuf, io::Error> {
    let root = State::path_for::<Source>().await?;
    debug!("Root: {:?}", root);
    Ok(PathBuf::from(root).join("config.yaml"))
}

async fn put_config(config_query: &ConfigQuery) -> Result<(), Error> {
    let config_path = config_path()
        .await
        .map_err(|e| e.to_source_creation_builder_error())?;

    debug!("Config path: {:?}", config_path);

    let serialized =
        serde_yaml::to_string(config_query).map_err(|e| e.to_yaml_serialization_error())?;

    fs::write(&config_path, serialized)
        .await
        .map_err(|e| e.to_source_creation_builder_error())?;

    Ok(())
}

pub async fn get_config() -> Option<ConfigQuery> {
    let config_path = config_path().await;

    if let Ok(config_path) = config_path {
        debug!("Config path: {:?}", config_path);
        let read_result = fs::read(&config_path).await;
        debug!("Read result: {:?}", read_result);
        if let Ok(config) = read_result {
            let parse_result = serde_yaml::from_slice(&config);
            debug!("Parse result: {:?}", parse_result);
            if let Ok(config) = parse_result {
                return Some(config);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::auth::gen_token_for_path;
    use crate::test::test::CleanableTestFile;
    use lazy_static::lazy_static;
    use std::sync::Mutex;
    use warp::http::header::COOKIE;
    use warp::http::StatusCode;
    use warp::test::request;

    lazy_static! {
        pub static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
    }

    async fn make_config_file_and_lock<'a>() -> CleanableTestFile<'a> {
        CleanableTestFile::new(
            TEST_MUTEX.lock().expect("Could not lock mutex."),
            config_path()
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
        let config_path = config_path().await.unwrap();
        let _ = fs::remove_file(&config_path).await;
    }

    fn test_config() -> ConfigQuery {
        ConfigQuery {
            project_id: "test_project_id".to_string(),
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            auth_uri: "test_auth_uri".to_string(),
            auth_provider_x509_cert_url: "test_auth_provider_x509_cert_url".to_string(),
            token_uri: "test_token_uri".to_string(),
        }
    }

    fn test_config_form_encoded() -> String {
        "project_id=test_project_id&\
        client_id=test_client_id&\
        client_secret=test_client_secret&\
        auth_uri=test_auth_uri&\
        auth_provider_x509_cert_url=test_auth_provider_x509_cert_url&\
        token_uri=test_token_uri"
            .to_string()
    }

    mod config_google {
        use super::*;
        use crate::core::node_handles::test::get_test_node_handles;
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

            let expected = format_config_google_html(&None).await;
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

            let expected = format_config_google_html(&Some(config)).await;
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
            assert!(actual.contains("name=\"auth_uri\" value=\"test_auth_uri\"\n"));
            assert!(actual.contains(
                "name=\"auth_provider_x509_cert_url\" value=\"test_auth_provider_x509_cert_url\"\n"
            ));
            assert!(actual.contains("name=\"token_uri\" value=\"test_token_uri\"\n"));
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

            #[test]
            fn returns_application_secret() {
                let config = test_config();
                let core_config = Config::with_all_properties(None, None, None, None, None);
                let application_secret = config.to_application_secret(&core_config);
                assert_eq!(application_secret.client_id, "test_client_id");
                assert_eq!(application_secret.client_secret, "test_client_secret");
                assert_eq!(application_secret.auth_uri, "test_auth_uri");
                assert_eq!(
                    application_secret.auth_provider_x509_cert_url,
                    Some("test_auth_provider_x509_cert_url".to_string())
                );
                assert_eq!(application_secret.token_uri, "test_token_uri");
                assert_eq!(
                    application_secret.redirect_uris,
                    vec!["https://localhost:443/sessions/auth/google"]
                );
                assert_eq!(
                    application_secret.project_id,
                    Some("test_project_id".to_string())
                );
                assert_eq!(application_secret.client_email, None);
                assert_eq!(application_secret.client_x509_cert_url, None);
            }
        }

        mod make_redirect_uris {
            use super::*;

            #[test]
            fn with_domain_config_returns_https() {
                let config = Config::with_all_properties(
                    Some(DomainConfig::new("test_domain".to_string())),
                    None,
                    None,
                    Some(8080),
                    Some("test".to_string()),
                );
                let redirect_uris = ConfigQuery::make_redirect_uris(&config);
                assert_eq!(redirect_uris.len(), 1);
                assert_eq!(
                    redirect_uris[0],
                    "https://test_domain:8080/sessions/auth/google"
                );
            }

            #[test]
            fn without_domain_config_returns_http() {
                let config = Config::with_all_properties(
                    None,
                    None,
                    None,
                    Some(8080),
                    Some("test_domain".to_string()),
                );
                let redirect_uris = ConfigQuery::make_redirect_uris(&config);
                assert_eq!(redirect_uris.len(), 1);
                assert_eq!(
                    redirect_uris[0],
                    "http://localhost:8080/sessions/auth/google"
                );
            }
        }
    }
}
