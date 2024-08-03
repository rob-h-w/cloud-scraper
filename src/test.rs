#[cfg(test)]
pub mod test {
    use std::future::Future;
    use std::sync::MutexGuard;

    pub struct CleanableTestFile<'a> {
        _guard: MutexGuard<'a, ()>,
        path: String,
    }

    impl<'a> CleanableTestFile<'a> {
        pub async fn new<ErrorType, ResponseFuture, SaveFunctionType>(
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
