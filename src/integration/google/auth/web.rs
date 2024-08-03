use crate::core::module::State;
use crate::integration::google::auth::Google;
use crate::server::auth::auth_validation;
use handlebars::Handlebars;
use log::debug;
use once_cell::sync::Lazy;
use paste::paste;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use tokio::fs;
use warp::{reply, Filter};

use crate::domain::module_state::ModuleState;
use crate::static_init::error::{Error, IoErrorExt, SerdeErrorExt};

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

pub fn config_google() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
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
            .and_then(update_config))
}

async fn format_response() -> Result<impl warp::Reply, warp::Rejection> {
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

    PAGE_TEMPLATE.render(CONFIG_TEMPLATE, &page_data).unwrap()
}

async fn update_config(
    form_map: HashMap<String, String>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let config = ConfigQuery::new(&form_map);

    match put_config(&config).await {
        Ok(_) => Ok(warp::redirect::found(warp::http::Uri::from_static(
            ROOT_PATH,
        ))),
        Err(e) => Err(warp::reject::custom(e)),
    }
}

async fn config_path() -> Result<PathBuf, io::Error> {
    let root = State::path_for::<Google>().await?;
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

    let wrote = fs::write(&config_path, serialized)
        .await
        .map_err(|e| e.to_source_creation_builder_error())?;
    debug!("Wrote: {:?}", wrote);

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

    mod config_google {
        use super::*;

        #[tokio::test]
        async fn with_none_returns_html_with_defaults() {
            let _lock = make_config_file_and_lock().await;

            reset().await;

            let token = gen_token_for_path("/");
            let filter = config_google();
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
            println!("{}", actual);

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
            let filter = config_google();
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
            println!("{}", actual);

            assert!(actual.contains("name=\"project_id\" value=\"test_project_id\"\n"));
            assert!(actual.contains("name=\"client_id\" value=\"test_client_id\"\n"));
            assert!(actual.contains("name=\"client_secret\" value=\"test_client_secret\"\n"));
            assert!(actual.contains("name=\"auth_uri\" value=\"test_auth_uri\"\n"));
            assert!(actual.contains(
                "name=\"auth_provider_x509_cert_url\" value=\"test_auth_provider_x509_cert_url\"\n"
            ));
            assert!(actual.contains("name=\"token_uri\" value=\"test_token_uri\"\n"));
        }
    }
}
