use crate::core::cli::Command::{RootPassword, Serve};
use crate::core::cli::{Cli, Command, ServeArgs};
use crate::core::construct_config;
use crate::core::engine::{Engine, EngineImpl};
use crate::core::root_password::{create_root_password, root_password_exists};
use crate::domain::config::Config;
use crate::server;
use crate::server::WebServer;
use clap::Parser;
use log::debug;
use std::sync::Arc;

pub async fn main_impl<Interface>() -> Result<(), String>
where
    Interface: MainInterface,
{
    // Make sure we never initialize the env_logger in unit tests.
    Interface::initialize_logging();

    debug!("Reading cli input...");
    let cli = Interface::get_cli();

    match &cli.command {
        Command::Config(config_args) => {
            construct_config(config_args).await;
        }
        RootPassword(_root_password_args) => {
            create_root_password().await?;
        }
        Serve(serve_args) => {
            debug!("Checking root password...");
            if !root_password_exists().await {
                return Err(
                    "Root password not set. Use root-password to create a password before running \
                    serve."
                        .to_string(),
                );
            }

            debug!("Reading config...");
            let config = Interface::construct_config(serve_args);

            debug!("Checking config...");
            config.sanity_check()?;

            debug!("Constructing server...");
            let server = Interface::construct_server(config.clone());

            debug!("Constructing engine...");
            let engine = Interface::construct_engine(config, server);

            debug!("Starting engine");
            engine.start().await;
        }
    }
    Ok(())
}

pub trait MainInterface {
    fn initialize_logging();
    fn get_cli() -> Cli;
    fn construct_config(cli: &ServeArgs) -> Arc<Config>;
    fn construct_server(config: Arc<Config>) -> impl WebServer;
    fn construct_engine(config: Arc<Config>, server: impl WebServer) -> impl Engine;
}

pub struct CoreInterface {}

impl MainInterface for CoreInterface {
    fn initialize_logging() {
        env_logger::init()
    }

    fn get_cli() -> Cli {
        Cli::parse()
    }

    fn construct_config(cli: &ServeArgs) -> Arc<Config> {
        Config::new(cli)
    }

    fn construct_server(config: Arc<Config>) -> impl WebServer {
        server::new(config)
    }

    fn construct_engine(config: Arc<Config>, server: impl WebServer) -> impl Engine {
        EngineImpl::new(&config, server)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_on;
    use crate::core::engine::MockEngine;
    use crate::core::root_password::tests::with_test_root_password_scope;
    use crate::domain::config::tests::{test_config, test_config_with};
    use crate::domain::config::DomainConfig;
    use crate::server::MockWebServer;
    use crate::test::{LogEntry, Logger};
    use lazy_static::lazy_static;
    use once_cell::sync::Lazy;
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};

    fn empty_cli() -> Cli {
        Cli {
            command: Serve(ServeArgs::default()),
        }
    }

    macro_rules! static_called {
        ($name:ident) => {
            paste::paste! {
                static [<$name:upper _CALLED>]: Lazy<Mutex<AtomicBool>> =
                    Lazy::new(|| Mutex::new(AtomicBool::new(false)));
            }
        };
    }
    macro_rules! get_called {
        ($CALLED:ident) => {
            paste::paste! {
                #[allow(dead_code)]
                pub fn [<is_ $CALLED:lower _called>]() -> bool {
                    let called = [<$CALLED:upper _CALLED>].lock().expect("Mutex poisoned");
                    called.load(std::sync::atomic::Ordering::SeqCst)
                }
            }
        };
    }
    macro_rules! with_verifiers {
        ( $name:ident ) => {
            static_called!(LOG_INITIALIZER);
            static_called!(CLI_GETTER);
            static_called!(CONFIG_CONSTRUCTOR);
            static_called!(SERVER_CONSTRUCTOR);
            static_called!(ENGINE_CONSTRUCTOR);
            struct $name {}
            impl $name {
                get_called!(LOG_INITIALIZER);
                get_called!(CLI_GETTER);
                get_called!(CONFIG_CONSTRUCTOR);
                get_called!(SERVER_CONSTRUCTOR);
                get_called!(ENGINE_CONSTRUCTOR);
            }
        };
    }

    macro_rules! note_called {
        ($CALLED:ident) => {
            paste::paste! {
                let called = [<$CALLED:upper _CALLED>].lock().expect("Mutex poisoned");
                called.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        };
    }

    macro_rules! with_empty_logging {
        () => {
            fn initialize_logging() {
                note_called!(LOG_INITIALIZER);
            }
        };
    }

    macro_rules! with_empty_cli {
        () => {
            fn get_cli() -> Cli {
                note_called!(CLI_GETTER);
                empty_cli()
            }
        };
    }

    lazy_static! {
        static ref TEST_CONFIG: Arc<Config> = test_config();
    }

    macro_rules! with_test_config {
        () => {
            fn construct_config(_serve_args: &ServeArgs) -> Arc<Config> {
                note_called!(CONFIG_CONSTRUCTOR);
                TEST_CONFIG.clone()
            }
            fn construct_server(_config: Arc<Config>) -> impl WebServer {
                note_called!(SERVER_CONSTRUCTOR);
                MockWebServer::new()
            }
            fn construct_engine(_config: Arc<Config>, _server: impl WebServer) -> impl Engine {
                note_called!(ENGINE_CONSTRUCTOR);
                let mut eng = MockEngine::new();

                eng.expect_start().times(1).returning(|| Box::pin(async {}));

                eng
            }
        };
    }

    lazy_static! {
        static ref INSANE_CONFIG: Arc<Config> =
            test_config_with(Some(DomainConfig::new("https://insane.domain")), None);
    }

    macro_rules! with_insane_config {
        () => {
            fn construct_config(_serve_args: &ServeArgs) -> Arc<Config> {
                note_called!(CONFIG_CONSTRUCTOR);
                INSANE_CONFIG.clone()
            }
            fn construct_server(_config: Arc<Config>) -> impl WebServer {
                note_called!(SERVER_CONSTRUCTOR);
                MockWebServer::new()
            }
            fn construct_engine(_config: Arc<Config>, _server: impl WebServer) -> impl Engine {
                note_called!(ENGINE_CONSTRUCTOR);
                let mut eng = MockEngine::new();

                eng.expect_start().times(1).returning(|| Box::pin(async {}));

                eng
            }
        };
    }

    macro_rules! with_password_file {
        ($b:block) => {
            let _scope = block_on!(with_test_root_password_scope());
            $b
        };
    }

    #[test]
    fn test_main_impl() {
        with_password_file!({
            with_verifiers!(MockMainInterface);
            impl MainInterface for MockMainInterface {
                with_empty_logging!();
                with_empty_cli!();
                with_test_config!();
            }
            let _ = block_on!(main_impl::<MockMainInterface>());

            assert_eq!(MockMainInterface::is_log_initializer_called(), true);
            assert_eq!(MockMainInterface::is_cli_getter_called(), true);
            assert_eq!(MockMainInterface::is_config_constructor_called(), true);
            assert_eq!(MockMainInterface::is_server_constructor_called(), true);
            assert_eq!(MockMainInterface::is_engine_constructor_called(), true);
        });
    }

    #[test]
    fn test_main_impl_calls_log_initializer() {
        with_password_file!({
            with_verifiers!(MockMainInterface);
            impl MainInterface for MockMainInterface {
                with_empty_logging!();
                with_empty_cli!();
                with_test_config!();
            }
            block_on!(main_impl::<MockMainInterface>()).unwrap();

            assert_eq!(MockMainInterface::is_log_initializer_called(), true);
            assert_eq!(MockMainInterface::is_cli_getter_called(), true);
            assert_eq!(MockMainInterface::is_config_constructor_called(), true);
            assert_eq!(MockMainInterface::is_server_constructor_called(), true);
            assert_eq!(MockMainInterface::is_engine_constructor_called(), true);
        });
    }

    #[test]
    fn test_main_impl_logs() {
        with_password_file!({
            with_verifiers!(MockMainInterface);
            impl MainInterface for MockMainInterface {
                fn initialize_logging() {
                    Logger::init()
                }
                with_empty_cli!();
                with_test_config!();
            }

            Logger::use_in(|logger| {
                block_on!(main_impl::<MockMainInterface>()).unwrap();
                assert_eq!(
                    logger.log_entry_exists(&LogEntry::debug("Starting engine")),
                    true
                );
            });
        });
    }

    #[test]
    fn test_barfs_insane_config() {
        with_password_file!({
            with_verifiers!(MockMainInterface);
            impl MainInterface for MockMainInterface {
                with_empty_logging!();
                with_empty_cli!();
                with_insane_config!();
            }

            let result = block_on!(main_impl::<MockMainInterface>());

            assert!(result.is_err());
        });
    }
}
