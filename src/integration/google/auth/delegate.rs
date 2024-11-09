use derive_builder::Builder;
use google_tasks1::common::auth::GetTokenClone;
use google_tasks1::common::GetToken;
use std::future::Future;
use std::pin::Pin;

#[derive(Builder)]
pub struct Delegate {}

impl Delegate {
    pub(crate) fn new() -> Self {
        Self {}
    }

    async fn get_token(
        &self,
        _scopes: &[&str],
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        todo!()
    }
}

impl GetTokenClone for Delegate {
    fn clone_box(&self) -> Box<dyn GetToken> {
        todo!()
    }
}

impl GetToken for Delegate {
    fn get_token<'a>(
        &'a self,
        scopes: &'a [&str],
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(self.get_token(scopes))
    }
}
