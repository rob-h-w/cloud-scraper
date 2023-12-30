use crate::domain::config::Config as DomainConfig;

pub(crate) struct Config {}

impl DomainConfig for Config {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instantiate() {
        Config {};
    }
}
