use crate::domain::oauth2::Client;
use google_tasks1::common::GetToken;
use std::future::Future;
use std::pin::Pin;

pub struct Delegate {
    client: Pin<Box<dyn Client>>,
}

impl Delegate {
    pub fn new(client: Pin<Box<dyn Client>>) -> Self {
        Self { client }
    }

    async fn get_secret(
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

impl Clone for Delegate {
    fn clone(&self) -> Self {
        Self {
            client: self.client.duplicate(),
        }
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
        Box::pin(self.get_secret(scopes))
    }
}
