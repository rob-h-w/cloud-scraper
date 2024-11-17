use crate::domain::oauth2::Client;
use derive_builder::Builder;
use google_tasks1::common::GetToken;
use std::future::Future;
use std::pin::Pin;

#[derive(Builder, Clone)]
pub struct Delegate {
    client: Client,
}

impl Delegate {
    async fn get_token(
        &self,
        scopes: &[&str],
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .get_token(scopes)
            .await
            .map(|token| Some(token.secret().clone()))
            .map_err(|e| e.into())
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
