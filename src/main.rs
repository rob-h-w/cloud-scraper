use std::error::Error;

use log::debug;

use crate::core::engine::{Engine, EngineImpl};

mod core;
mod domain;
mod integration;

fn main() -> Result<(), Box<dyn Error>> {
    main_impl(&mut EngineImpl {}, env_logger::init)
}

fn main_impl<T>(engine: &mut impl Engine, log_initializer: T) -> Result<(), Box<dyn Error>>
where
    T: FnOnce() -> (),
{
    // Make sure we never initialize the env_logger in unit tests.
    log_initializer();

    debug!("Starting engine");
    engine.start()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::sync::Once;
    use std::sync::{Arc, Mutex};

    use log::{Log, Metadata, Record};
    use parking_lot::ReentrantMutex;

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
        start_called: bool,
    }

    impl EngineTestImpl {
        fn new() -> EngineTestImpl {
            EngineTestImpl {
                start_called: false,
            }
        }
    }

    impl Engine for EngineTestImpl {
        fn start(&mut self) -> Result<(), Box<dyn Error>> {
            self.start_called = true;
            Ok(())
        }
    }

    #[test]
    fn test_main_impl() {
        let mut e = EngineTestImpl::new();
        let log_initializer = || -> () {};
        main_impl(&mut e, log_initializer).unwrap();

        assert_eq!(e.start_called, true);
    }

    #[test]
    fn test_main_impl_calls_log_initializer() {
        let mut e = EngineTestImpl::new();
        let mut log_initializer_called = false;
        let log_initializer = || -> () {
            log_initializer_called = true;
        };
        main_impl(&mut e, log_initializer).unwrap();

        assert_eq!(log_initializer_called, true);
    }

    #[test]
    fn test_main_impl_logs() {
        let mut e = EngineTestImpl::new();

        Logger::use_in(|logger| {
            main_impl(&mut e, Logger::init).unwrap();
            assert_eq!(
                logger.log_entry_exists(&LogEntry::debug("Starting engine")),
                true
            );
        });
    }
}
