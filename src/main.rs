use clap::Parser;
use log::debug;
use std::sync::Arc;

use crate::core::cli::Cli;
use crate::core::config::Config;
use crate::core::engine::{Engine, EngineImpl};
use crate::domain::config::Config as DomainConfig;
use server::WebServer;

mod core;
mod domain;
mod integration;
mod macros;
mod server;
mod static_init;

#[tokio::main]
async fn main() -> Result<(), String> {
    main_impl(
        env_logger::init,
        Cli::parse,
        Config::new,
        server::new,
        EngineImpl::new,
    )
    .await
}

async fn main_impl<
    ConfigType,
    EngineType,
    ServerType,
    LogInitializer,
    CliGetter,
    ConfigConstructor,
    ServerConstructor,
    EngineConstructor,
>(
    log_initializer: LogInitializer,
    cli_getter: CliGetter,
    config_constructor: ConfigConstructor,
    server_constructor: ServerConstructor,
    engine_constructor: EngineConstructor,
) -> Result<(), String>
where
    ConfigType: DomainConfig,
    EngineType: Engine,
    ServerType: WebServer,
    LogInitializer: FnOnce(),
    CliGetter: FnOnce() -> Cli,
    ConfigConstructor: FnOnce(&Cli) -> Arc<ConfigType>,
    ServerConstructor: FnOnce(Arc<ConfigType>) -> ServerType,
    EngineConstructor: FnOnce(Arc<ConfigType>, ServerType) -> EngineType,
{
    // Make sure we never initialize the env_logger in unit tests.
    log_initializer();

    debug!("Reading cli input...");
    let cli = cli_getter();

    debug!("Reading config...");
    let config = config_constructor(&cli);

    debug!("Checking config...");
    config.sanity_check()?;

    debug!("Constructing server...");
    let server = server_constructor(config.clone());

    debug!("Constructing engine...");
    let engine = engine_constructor(config, server);

    debug!("Starting engine");
    engine.start().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::sync::Once;
    use std::sync::{Arc, Mutex};

    use crate::core::engine::MockEngine;
    use crate::domain::config::tests::TestConfig;
    use crate::domain::config::PipelineConfig;
    use crate::domain::source_identifier::SourceIdentifier;
    use crate::server::MockWebServer;
    use log::{Log, Metadata, Record};
    use parking_lot::ReentrantMutex;
    use serde_yaml::Value;

    use super::*;

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

    struct InsaneConfig {
        pipelines: Vec<PipelineConfig>,
    }

    impl DomainConfig for InsaneConfig {
        fn domain_config(&self) -> Option<&domain::config::DomainConfig> {
            None
        }

        fn sink(&self, _sink_identifier: &str) -> Option<&Value> {
            None
        }

        fn source(&self, _source_identifier: &SourceIdentifier) -> Option<&Value> {
            None
        }

        fn pipelines(&self) -> &Vec<PipelineConfig> {
            &self.pipelines
        }

        fn port(&self) -> u16 {
            80
        }

        fn sink_names(&self) -> Vec<String> {
            vec![]
        }

        fn sink_configured(&self, _name: &str) -> bool {
            false
        }

        fn site_folder(&self) -> &str {
            "insane_site_folder"
        }

        fn source_configured(&self, _name: &str) -> bool {
            false
        }

        fn sanity_check(&self) -> Result<(), String> {
            Err("Insane config".to_string())
        }
    }

    #[test]
    fn test_main_impl() {
        let cli = Cli {
            command: None,
            config: None,
            exit_after: None,
            port: None,
        };
        let config = Arc::new(TestConfig::new(None));
        let log_initializer = || -> () {};
        let cli_getter = || -> Cli { cli };
        let config_constructor = |_: &Cli| -> Arc<TestConfig> { config };
        let server_constructor = |_: Arc<TestConfig>| -> MockWebServer { MockWebServer::new() };
        let mut mock_engine = MockEngine::new();
        mock_engine
            .expect_start()
            .times(1)
            .returning(|| Box::pin(async {}));
        let engine_constructor =
            |_: Arc<TestConfig>, _: MockWebServer| -> MockEngine { mock_engine };
        block_on!(main_impl(
            log_initializer,
            cli_getter,
            config_constructor,
            server_constructor,
            engine_constructor
        ))
        .unwrap();
    }

    #[test]
    fn test_main_impl_calls_log_initializer() {
        let cli = Cli {
            command: None,
            config: None,
            exit_after: None,
            port: None,
        };
        let config = Arc::new(TestConfig::new(None));
        let mut log_initializer_called = false;
        let log_initializer = || -> () {
            log_initializer_called = true;
        };
        let cli_getter = || -> Cli { cli };
        let mut config_constructor_called = false;
        let config_constructor = |_: &Cli| -> Arc<TestConfig> {
            config_constructor_called = true;
            config
        };
        let server_constructor = |_: Arc<TestConfig>| -> MockWebServer { MockWebServer::new() };
        let mut mock_engine = MockEngine::new();
        mock_engine
            .expect_start()
            .times(1)
            .returning(|| Box::pin(async {}));
        let mut engine_constructor_called = false;
        let engine_constructor = |_: Arc<TestConfig>, _: MockWebServer| -> MockEngine {
            engine_constructor_called = true;
            mock_engine
        };
        block_on!(main_impl(
            log_initializer,
            cli_getter,
            config_constructor,
            server_constructor,
            engine_constructor
        ))
        .unwrap();

        assert_eq!(log_initializer_called, true);
        assert_eq!(config_constructor_called, true);
        assert_eq!(engine_constructor_called, true);
    }

    #[test]
    fn test_main_impl_logs() {
        let cli = Cli {
            command: None,
            config: None,
            exit_after: None,
            port: None,
        };
        let config = Arc::new(TestConfig::new(None));
        let cli_getter = || -> Cli { cli };
        let config_constructor = |_: &Cli| -> Arc<TestConfig> { config };
        let server_constructor = |_: Arc<TestConfig>| -> MockWebServer { MockWebServer::new() };
        let mut mock_engine = MockEngine::new();
        mock_engine
            .expect_start()
            .times(1)
            .returning(|| Box::pin(async {}));
        let engine_constructor =
            |_: Arc<TestConfig>, _: MockWebServer| -> MockEngine { mock_engine };

        Logger::use_in(|logger| {
            block_on!(main_impl(
                Logger::init,
                cli_getter,
                config_constructor,
                server_constructor,
                engine_constructor
            ))
            .unwrap();
            assert_eq!(
                logger.log_entry_exists(&LogEntry::debug("Starting engine")),
                true
            );
        });
    }

    #[test]
    fn test_barfs_insane_config() {
        let cli = Cli {
            command: None,
            config: None,
            exit_after: None,
            port: None,
        };
        let config = Arc::new(InsaneConfig { pipelines: vec![] });
        let cli_getter = || -> Cli { cli };
        let config_constructor = |_: &Cli| -> Arc<InsaneConfig> { config };
        let server_constructor = |_: Arc<InsaneConfig>| -> MockWebServer { MockWebServer::new() };
        let engine_constructor =
            |_: Arc<InsaneConfig>, _: MockWebServer| -> MockEngine { MockEngine::new() };

        let result = block_on!(main_impl(
            Logger::init,
            cli_getter,
            config_constructor,
            server_constructor,
            engine_constructor
        ));

        assert!(result.is_err());
    }
}
