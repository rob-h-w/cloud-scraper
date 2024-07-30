use crate::core::cli::{Cli, Command, ServeArgs};
use crate::core::config::Config;
use crate::core::engine::{Engine, EngineImpl};
use crate::domain::config::Config as DomainConfig;
use clap::Parser;
use core::root_password::create_root_password;
use log::debug;
use server::WebServer;
use std::sync::Arc;
use Command::{RootPassword, Serve};

pub mod core;
mod domain;
mod integration;
mod macros;
mod server;

#[tokio::main]
async fn main() -> Result<(), String> {
    main_impl::<Config, CoreInterface>().await
}

async fn main_impl<ConfigType, Interface>() -> Result<(), String>
where
    ConfigType: DomainConfig,
    Interface: MainInterface<ConfigType>,
{
    // Make sure we never initialize the env_logger in unit tests.
    Interface::initialize_logging();

    debug!("Reading cli input...");
    let cli = Interface::get_cli();

    match &cli.command {
        RootPassword(_root_password_args) => {
            create_root_password().await?;
        }
        Serve(serve_args) => {
            debug!("Checking root password...");
            if !core::root_password::root_password_exists().await {
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

trait MainInterface<ConfigType>
where
    ConfigType: DomainConfig,
{
    fn initialize_logging();
    fn get_cli() -> Cli;
    fn construct_config(cli: &ServeArgs) -> Arc<ConfigType>;
    fn construct_server(config: Arc<ConfigType>) -> impl WebServer;
    fn construct_engine(config: Arc<ConfigType>, server: impl WebServer) -> impl Engine;
}

struct CoreInterface {}

impl MainInterface<Config> for CoreInterface {
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
        EngineImpl::new(config, server)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::engine::MockEngine;
    use crate::domain::config::tests::TestConfig;
    use crate::server::MockWebServer;
    use log::{Log, Metadata, Record};
    use once_cell::sync::Lazy;
    use parking_lot::ReentrantMutex;
    use std::cell::RefCell;
    use std::sync::atomic::AtomicBool;
    use std::sync::Once;
    use std::sync::{Arc, Mutex};

    static mut LOGGER: Option<Logger> = None;

    #[derive(Clone, Debug, PartialEq)]
    pub(crate) struct LogEntry {
        args: String,
        level: log::Level,
    }

    impl LogEntry {
        fn new(args: &str, level: log::Level) -> LogEntry {
            LogEntry {
                args: args.to_string(),
                level,
            }
        }

        fn debug(args: &str) -> LogEntry {
            LogEntry::new(args, log::Level::Debug)
        }

        pub(crate) fn args(&self) -> &str {
            &self.args
        }

        pub(crate) fn level(&self) -> log::Level {
            self.level
        }
    }

    pub(crate) struct Logger {
        records: Arc<ReentrantMutex<RefCell<Vec<LogEntry>>>>,
    }

    impl Logger {
        fn get() -> &'static mut Logger {
            unsafe {
                static mut LOGGER_INIT: Mutex<()> = Mutex::new(());
                let _guard = LOGGER_INIT.lock().unwrap();

                match LOGGER {
                    Some(ref mut l) => l,
                    None => {
                        LOGGER = Some(Logger::new());
                        LOGGER.as_mut().unwrap()
                    }
                }
            }
        }

        pub(crate) fn init() {
            static INIT: Once = Once::new();
            INIT.call_once(|| {
                log::set_logger(Logger::get()).unwrap();
                log::set_max_level(log::LevelFilter::Trace);
            });
        }

        pub(crate) fn use_in<T>(log_use: T)
        where
            T: FnOnce(&mut Logger) -> (),
        {
            let mutex = Logger::get().records.clone();
            let _lock = mutex.lock();
            log_use(Logger::get());
        }

        pub(crate) fn new() -> Self {
            Self {
                records: Arc::new(ReentrantMutex::from(RefCell::new(Vec::new()))),
            }
        }

        pub(crate) fn log_entries(&mut self) -> Vec<LogEntry> {
            self.records
                .lock()
                .borrow()
                .iter()
                .map(|r| r.clone())
                .collect()
        }

        fn log_entry_exists(&self, entry: &LogEntry) -> bool {
            let records = self.records.lock();
            let exists = records
                .borrow()
                .iter()
                .any(|r| r.args == entry.args() && r.level == entry.level());
            exists
        }

        pub(crate) fn reset(&mut self) {
            self.records.lock().borrow_mut().clear();
        }
    }

    impl Log for Logger {
        fn enabled(&self, _metadata: &Metadata) -> bool {
            true
        }

        fn log(&self, record: &Record) {
            unsafe {
                (*self.records.lock().as_ptr()).push(LogEntry::new(
                    record.args().to_string().as_str(),
                    record.level(),
                ));
            }
        }

        fn flush(&self) {}
    }

    struct InsaneConfig {}

    impl DomainConfig for InsaneConfig {
        fn domain_config(&self) -> Option<&domain::config::DomainConfig> {
            None
        }

        fn email(&self) -> Option<&str> {
            None
        }

        fn port(&self) -> u16 {
            80
        }

        fn site_folder(&self) -> &str {
            "insane_site_folder"
        }

        fn sanity_check(&self) -> Result<(), String> {
            Err("Insane config".to_string())
        }
    }

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

    static TEST_CONFIG: Lazy<Arc<TestConfig>> =
        Lazy::new(|| Arc::new(TestConfig::new_domain_email(None, None)));

    macro_rules! with_test_config {
        () => {
            fn construct_config(_serve_args: &ServeArgs) -> Arc<TestConfig> {
                note_called!(CONFIG_CONSTRUCTOR);
                TEST_CONFIG.clone()
            }
            fn construct_server(_config: Arc<TestConfig>) -> impl WebServer {
                note_called!(SERVER_CONSTRUCTOR);
                MockWebServer::new()
            }
            fn construct_engine(_config: Arc<TestConfig>, _server: impl WebServer) -> impl Engine {
                note_called!(ENGINE_CONSTRUCTOR);
                let mut eng = MockEngine::new();

                eng.expect_start().times(1).returning(|| Box::pin(async {}));

                eng
            }
        };
    }

    static INSANE_CONFIG: Lazy<Arc<InsaneConfig>> = Lazy::new(|| Arc::new(InsaneConfig {}));

    macro_rules! with_insane_config {
        () => {
            fn construct_config(_serve_args: &ServeArgs) -> Arc<InsaneConfig> {
                note_called!(CONFIG_CONSTRUCTOR);
                INSANE_CONFIG.clone()
            }
            fn construct_server(_config: Arc<InsaneConfig>) -> impl WebServer {
                note_called!(SERVER_CONSTRUCTOR);
                MockWebServer::new()
            }
            fn construct_engine(
                _config: Arc<InsaneConfig>,
                _server: impl WebServer,
            ) -> impl Engine {
                note_called!(ENGINE_CONSTRUCTOR);
                let mut eng = MockEngine::new();

                eng.expect_start().times(1).returning(|| Box::pin(async {}));

                eng
            }
        };
    }

    use crate::core::root_password::test::with_test_root_password_scope;

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
            impl MainInterface<TestConfig> for MockMainInterface {
                with_empty_logging!();
                with_empty_cli!();
                with_test_config!();
            }
            let _ = block_on!(main_impl::<TestConfig, MockMainInterface>());

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
            impl MainInterface<TestConfig> for MockMainInterface {
                with_empty_logging!();
                with_empty_cli!();
                with_test_config!();
            }
            block_on!(main_impl::<TestConfig, MockMainInterface>()).unwrap();

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
            impl MainInterface<TestConfig> for MockMainInterface {
                fn initialize_logging() {
                    Logger::init()
                }
                with_empty_cli!();
                with_test_config!();
            }

            Logger::use_in(|logger| {
                block_on!(main_impl::<TestConfig, MockMainInterface>()).unwrap();
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
            impl MainInterface<InsaneConfig> for MockMainInterface {
                with_empty_logging!();
                with_empty_cli!();
                with_insane_config!();
            }

            let result = block_on!(main_impl::<InsaneConfig, MockMainInterface>());

            assert!(result.is_err());
        });
    }
}
