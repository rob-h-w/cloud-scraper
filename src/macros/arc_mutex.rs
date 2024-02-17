#[macro_export]
macro_rules! arx {
    ($type:ty) => {
        Arc<Mutex<$type>>
    };
}
