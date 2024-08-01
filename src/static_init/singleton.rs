use std::future::Future;
use tokio::sync::RwLock;

pub(crate) async fn async_ginit<Content, Error, Fut>(
    singleton: &RwLock<Option<Content>>,
    initializer: impl FnOnce() -> Fut,
) -> Result<Content, Error>
where
    Content: Clone,
    Fut: Future<Output = Result<Content, Error>>,
{
    {
        let read_lock = singleton.read().await;

        if let Some(content) = read_lock.as_ref() {
            return Ok(content.clone());
        }
    }

    let mut write_lock = singleton.write().await;

    if write_lock.is_some() {
        return Ok(write_lock.as_ref().unwrap().clone());
    }

    write_lock.replace(initializer().await?);

    Ok(write_lock.as_ref().unwrap().clone())
}

#[cfg(test)]
pub(crate) async fn reset<Content>(singleton: &RwLock<Option<Content>>)
where
    Content: Clone,
{
    {
        let read_lock = singleton.read().await;

        if read_lock.is_none() {
            return;
        }
    }

    let mut write_lock = singleton.write().await;

    if write_lock.is_none() {
        return;
    }

    *write_lock = None;
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio_test::assert_ok;

    #[tokio::test]
    async fn test_async_ginit() {
        let singleton = RwLock::new(None);

        let result: Result<i32, String> =
            async_ginit(&singleton, || async { Ok::<i32, String>(42) }).await;

        assert_ok!(result);
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_reset() {
        let singleton = RwLock::new(Some(42));

        reset(&singleton).await;

        let read_lock = singleton.read().await;

        assert!(read_lock.is_none());
    }
}
