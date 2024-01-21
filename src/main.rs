use std::error::Error;
use std::sync::Arc;

use log::debug;

use crate::core::config::Config;
use crate::core::engine::{Engine, EngineImpl};
use crate::domain::config::Config as DomainConfig;

mod core;
mod domain;
mod integration;
mod macros;
mod static_init;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    main_impl(env_logger::init, Config::new, EngineImpl::new).await
}

async fn main_impl<ConfigType, EngineType, LogInitializer, ConfigGetter, EngineConstructor>(
    log_initializer: LogInitializer,
    config_getter: ConfigGetter,
    engine_constructor: EngineConstructor,
) -> Result<(), Box<dyn Error>>
where
    ConfigType: DomainConfig,
    EngineType: Engine<ConfigType>,
    LogInitializer: FnOnce(),
    ConfigGetter: FnOnce() -> Arc<ConfigType>,
    EngineConstructor: FnOnce(Arc<ConfigType>) -> Box<EngineType>,
{
    // Make sure we never initialize the env_logger in unit tests.
    log_initializer();

    debug!("Reading config...");
    let config = config_getter();

    debug!("Checking config...");
    config.sanity_check()?;

    debug!("Constructing engine...");
    let engine = engine_constructor(config);

    debug!("Starting engine");
    engine.start().await
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::sync::Once;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use log::{Log, Metadata, Record};
    use parking_lot::ReentrantMutex;
    use serde_yaml::Value;

    use crate::domain::config::tests::TestConfig;
    use crate::domain::config::PipelineConfig;
    use crate::domain::source_identifier::SourceIdentifier;

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

    struct EngineTestImpl {
        pub(crate) start_called: Arc<Mutex<bool>>,
    }

    #[async_trait]
    impl<ConfigType> Engine<ConfigType> for EngineTestImpl
    where
        ConfigType: DomainConfig,
    {
        fn new(_: Arc<ConfigType>) -> Box<Self> {
            Box::new(EngineTestImpl {
                start_called: Arc::new(Mutex::new(false)),
            })
        }

        async fn start(&self) -> Result<(), Box<dyn Error>> {
            *self.start_called.lock().unwrap() = true;
            Ok(())
        }
    }

    struct InsaneConfig {
        pipelines: Vec<PipelineConfig>,
    }

    impl DomainConfig for InsaneConfig {
        fn sink(&self, _sink_identifier: &str) -> Option<&Value> {
            None
        }

        fn source(&self, _source_identifier: &SourceIdentifier) -> Option<&Value> {
            None
        }

        fn pipelines(&self) -> &Vec<PipelineConfig> {
            &self.pipelines
        }

        fn sink_names(&self) -> Vec<String> {
            vec![]
        }

        fn sink_configured(&self, _name: &str) -> bool {
            false
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
        let config = Arc::new(TestConfig::new(None));
        let mut started: Option<Arc<Mutex<bool>>> = None;
        let log_initializer = || -> () {};
        let config_getter = || -> Arc<TestConfig> { config };
        let engine_constructor = |config: Arc<TestConfig>| -> Box<EngineTestImpl> {
            let engine = EngineTestImpl::new(config);
            started = Some(engine.start_called.clone());
            engine
        };
        block_on!(main_impl(
            log_initializer,
            config_getter,
            engine_constructor
        ))
        .unwrap();

        assert_eq!(*started.unwrap().lock().unwrap(), true);
    }

    #[test]
    fn test_main_impl_calls_log_initializer() {
        let config = Arc::new(TestConfig::new(None));
        let mut started: Option<Arc<Mutex<bool>>> = None;
        let mut log_initializer_called = false;
        let log_initializer = || -> () {
            log_initializer_called = true;
        };
        let mut config_getter_called = false;
        let config_getter = || -> Arc<TestConfig> {
            config_getter_called = true;
            config
        };
        let mut engine_constructor_called = false;
        let engine_constructor = |config: Arc<TestConfig>| -> Box<EngineTestImpl> {
            engine_constructor_called = true;
            let engine = EngineTestImpl::new(config);
            started = Some(engine.start_called.clone());
            engine
        };
        block_on!(main_impl(
            log_initializer,
            config_getter,
            engine_constructor
        ))
        .unwrap();

        assert_eq!(log_initializer_called, true);
        assert_eq!(config_getter_called, true);
        assert_eq!(engine_constructor_called, true);

        assert_eq!(*started.unwrap().lock().unwrap(), true);
    }

    #[test]
    fn test_main_impl_logs() {
        let config = Arc::new(TestConfig::new(None));
        let config_getter = || -> Arc<TestConfig> { config };

        Logger::use_in(|logger| {
            block_on!(main_impl(Logger::init, config_getter, EngineTestImpl::new)).unwrap();
            assert_eq!(
                logger.log_entry_exists(&LogEntry::debug("Starting engine")),
                true
            );
        });
    }

    #[test]
    fn test_barfs_insane_config() {
        let config = Arc::new(InsaneConfig { pipelines: vec![] });
        let config_getter = || -> Arc<InsaneConfig> { config };

        let result = block_on!(main_impl(Logger::init, config_getter, EngineTestImpl::new));

        assert!(result.is_err());
    }
}
