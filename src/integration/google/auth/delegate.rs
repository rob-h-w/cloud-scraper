use crate::domain::oauth2::Client;
use derive_builder::Builder;
use google_tasks1::common::GetToken;
use std::future::Future;
use std::pin::Pin;

#[derive(Builder, Clone)]
pub struct Delegate<ClientImpl>
where
    ClientImpl: Client,
{
    client: ClientImpl,
}

impl<ClientImpl: Client> Delegate<ClientImpl>
where
    ClientImpl: Client,
{
    async fn get_secret<'a>(
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

impl<ClientImpl: Client> GetToken for Delegate<ClientImpl>
where
    ClientImpl: Client,
{
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
        Box::pin(self.get_secret(scopes))
    }
}
