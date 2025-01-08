use crate::domain::module_state::ModuleState;
use async_trait::async_trait;

pub struct State {}

#[async_trait]
impl ModuleState for State {
    fn path() -> &'static str {
        "state"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::oauth2::BasicClientImpl;
    use crate::integration::google::Source;

    #[tokio::test]
    async fn test_path_for() {
        assert_eq!(State::path(), "state");
        assert_eq!(
            State::path_for::<Source<BasicClientImpl>>().await.unwrap(),
            "state/google"
        )
    }
}
