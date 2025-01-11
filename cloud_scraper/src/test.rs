#[cfg(test)]
pub(crate) use crate::test::tests::LogEntry;
#[cfg(test)]
pub(crate) use crate::test::tests::Logger;
#[cfg(test)]
pub(crate) mod tests {
    use lazy_static::lazy_static;
    use log::{Log, Metadata, Record};
    use parking_lot::ReentrantMutex;
    use std::cell::RefCell;
    use std::future::Future;
    use std::ops::Deref;
    use std::sync::{Arc, MutexGuard, Once};

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

        pub(crate) fn debug(args: &str) -> LogEntry {
            LogEntry::new(args, log::Level::Debug)
        }

        pub(crate) fn args(&self) -> &str {
            &self.args
        }

        pub(crate) fn level(&self) -> log::Level {
            self.level
        }
    }

    pub(crate) struct LogAccessor {
        records: Arc<ReentrantMutex<RefCell<Vec<LogEntry>>>>,
    }

    impl LogAccessor {
        fn new() -> Self {
            Self {
                records: Arc::new(ReentrantMutex::from(RefCell::new(Vec::new()))),
            }
        }

        pub(crate) fn log_entries(&self) -> Vec<LogEntry> {
            self.records
                .lock()
                .borrow()
                .iter()
                .map(|r| r.clone())
                .collect()
        }

        pub(crate) fn log_entry_exists(&self, entry: &LogEntry) -> bool {
            let records = self.records.lock();
            let exists = records
                .borrow()
                .iter()
                .any(|r| r.args == entry.args() && r.level == entry.level());
            exists
        }

        pub(crate) fn reset(&self) {
            self.records.lock().borrow_mut().clear();
        }
    }

    lazy_static! {
        static ref LOGGER: Logger = Logger::new();
    }

    pub(crate) struct Logger {
        log_accessor: Arc<LogAccessor>,
    }

    impl Logger {
        pub(crate) fn init() {
            static INIT: Once = Once::new();
            INIT.call_once(|| {
                log::set_logger(LOGGER.deref()).unwrap();
                log::set_max_level(log::LevelFilter::Trace);
            });
        }

        fn new() -> Self {
            Self {
                log_accessor: Arc::new(LogAccessor::new()),
            }
        }

        pub(crate) fn use_in<T>(log_use: T)
        where
            T: FnOnce(&LogAccessor) -> (),
        {
            let _guard = LOGGER.log_accessor.records.lock();
            let log_accessor = LOGGER.log_accessor.clone();
            log_use(log_accessor.as_ref());
        }
    }

    impl Log for Logger {
        fn enabled(&self, _metadata: &Metadata) -> bool {
            true
        }

        fn log(&self, record: &Record) {
            let lock = self.log_accessor.records.lock();
            lock.borrow_mut().push(LogEntry::new(
                record.args().to_string().as_str(),
                record.level(),
            ));
        }

        fn flush(&self) {}
    }

    pub(crate) struct CleanableTestFile<'a> {
        _guard: MutexGuard<'a, ()>,
        path: String,
    }

    impl<'a> CleanableTestFile<'a> {
        pub(crate) async fn new<ErrorType, ResponseFuture, SaveFunctionType>(
            guard: MutexGuard<'a, ()>,
            path: String,
            save_function: SaveFunctionType,
        ) -> Self
        where
            ResponseFuture: Future<Output = Result<(), ErrorType>> + Sized,
            SaveFunctionType: Fn(String) -> ResponseFuture,
            ErrorType: std::fmt::Debug,
        {
            save_function(path.clone())
                .await
                .expect(&format!("Could not create {:?}", path));
            Self {
                _guard: guard,
                path: path.to_string(),
            }
        }

        #[allow(dead_code)]
        fn drop(&self) {
            std::fs::remove_file(&self.path).expect("Could not remove root password file.");
        }
    }
}
