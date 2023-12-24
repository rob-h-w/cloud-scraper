use std::error::Error;
use crate::core::engine::{EngineImpl, Engine};

mod core;

fn main() -> Result<(), Box<dyn Error>> {
    main_impl(EngineImpl {})
}

fn main_impl(engine: impl Engine) -> Result<(), Box<dyn Error>> {
    engine.start()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EngineTestImpl {
        start_called: bool
    }
    impl EngineTestImpl {
        fn new() -> EngineTestImpl {
            EngineTestImpl { start_called: false }
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
        let e = EngineTestImpl::new();
        main_impl(e)?;

        assert_eq!(e.start_called, true)
    }
}
