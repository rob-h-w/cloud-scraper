use serde_yaml::Value;

#[derive(Debug)]
pub(crate) struct KeepSource {}

impl KeepSource {
    pub(crate) fn new(_config: &Value) -> Self {
        Self {}
    }
}

#[cfg(test)]
mod tests {}
