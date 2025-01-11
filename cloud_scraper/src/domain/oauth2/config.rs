use paste::paste;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::path::Path;

macro_rules! make_config {
    ($the_trait:ident, { $($e:ident),* }) => {
        paste! {
            pub trait $the_trait: Debug + Deserialize<'static> + From<&'static HashMap<String, String>> + Serialize {
                $(
                    fn $e(&self) -> &str;
                )*
            }
        }
    };
}

macro_rules! make_config_struct {
    ($struct:ident, $the_trait:ident, { $($e:ident),* }, { $($d:ident, $v:literal),* }) => {
        paste! {
            #[derive(Builder, Debug, Deserialize, Serialize)]
            pub struct $struct {
                $(
                    $e: String,
                )*
                $(
                    $d: String,
                )*
            }

            impl $the_trait for $struct {
                $(
                    fn $e(&self) -> &str {
                        &self.$e
                    }
                )*

                $(
                    fn $d(&self) -> &str {
                        &self.$d
                    }
                )*
            }

            impl From<&HashMap<String, String>> for $struct {
                fn from(map: &HashMap<String, String>) -> Self {
                    Self {
                        $(
                            $e: map.get(stringify!($e)).expect(&Self::format_missing_hash_key_message(stringify!($e), map)).clone(),
                        )*
                        $(
                            $d: map.get(stringify!($d)).unwrap_or(&String::from($v)).clone(),
                        )*
                    }
                }
            }

            impl $struct {
                fn format_missing_hash_key_message(key: &str, map: &HashMap<String, String>) -> String {
                    format!("Could not get required value {} from {:?}", key, map)
                }
            }
        }
    };
}

pub(crate) use make_config_struct;

make_config!(
    Config,
    {
        auth_uri,
        auth_provider_x509_cert_url,
        client_id,
        client_secret,
        project_id,
        token_uri
    }
);

pub trait PersistableConfig: Config {
    fn persist(
        &self,
        path: &Path,
    ) -> impl Future<Output = Result<(), std::io::Error>> + Send + Sync;
    fn read_config(path: &Path)
        -> impl Future<Output = Result<Self, std::io::Error>> + Send + Sync;
}

#[cfg(test)]
mod test {
    use super::*;
    use derive_builder::Builder;

    make_config!(
        TestConfig,
        { a, b }
    );

    make_config_struct!(
        TestConfigStruct, TestConfig, { a }, { b, "c" }
    );

    #[test]
    fn constructor_works() {
        let config: TestConfigStruct = TestConfigStructBuilder::default()
            .a("a".into())
            .b("b".into())
            .build()
            .expect("Failed to build TestConfigStruct");
        assert_eq!(config.a(), "a");
        assert_eq!(config.b(), "b");
    }

    #[test]
    fn from_hash_map_works() {
        let mut map = HashMap::new();
        map.insert("a".into(), "a".into());
        let config: TestConfigStruct = (&map).into();
        assert_eq!(config.a(), "a");
        assert_eq!(config.b(), "c");
    }

    #[test]
    #[should_panic]
    fn from_hash_map_missing_key() {
        let map = HashMap::new();
        let _config: TestConfigStruct = (&map).into();
    }
}
