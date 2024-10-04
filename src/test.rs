#[cfg(test)]
pub(crate) use crate::test::test::LogEntry;
#[cfg(test)]
pub(crate) use crate::test::test::Logger;
#[cfg(test)]
pub(crate) mod test {
    use log::{Log, Metadata, Record};
    use parking_lot::ReentrantMutex;
    use std::cell::RefCell;
    use std::future::Future;
    use std::sync::{Arc, Mutex, MutexGuard, Once};

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

    static mut LOGGER: Option<Logger> = None;

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

        pub(crate) fn log_entry_exists(&self, entry: &LogEntry) -> bool {
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
