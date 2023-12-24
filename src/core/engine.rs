use std::error::Error;

pub(crate) struct EngineImpl {
}

pub(crate) trait Engine {
    fn start(&self) -> Result<(), Box<dyn Error>>;
}

impl Engine for EngineImpl {
    fn start(&self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_start() {
        EngineImpl {}.start()?;
    }
}
