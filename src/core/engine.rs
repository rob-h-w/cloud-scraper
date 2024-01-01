use crate::domain::config::Config;
use std::error::Error;
use std::rc::Rc;

pub(crate) struct EngineImpl<T>
where
    T: Config,
{
    _config: Rc<T>,
}

pub(crate) trait Engine<T>
where
    T: Config,
{
    fn new(config: Rc<T>) -> Box<Self>;
    fn start(&mut self) -> Result<(), Box<dyn Error>>;
}

impl<T> Engine<T> for EngineImpl<T>
where
    T: Config,
{
    fn new(config: Rc<T>) -> Box<EngineImpl<T>> {
        Box::new(EngineImpl { _config: config })
    }

    fn start(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::tests::TestConfig;

    #[test]
    fn test_engine_start() {
        EngineImpl::new(Rc::new(TestConfig::new(None)))
            .start()
            .unwrap();
    }
}
