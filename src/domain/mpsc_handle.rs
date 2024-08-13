use crate::domain::node::InitReplier;
use async_trait::async_trait;
use log::{error, trace};
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

pub fn one_shot<T>() -> (OneshotMpscSenderHandle<T>, OneshotMpscReceiverHandle<T>) {
    let (sender, receiver) = mpsc::channel::<T>(1);
    (
        OneshotMpscSenderHandle::new(sender),
        OneshotMpscReceiverHandle::new(receiver),
    )
}

#[derive(Clone, Debug)]
pub struct OneshotMpscSenderHandle<T> {
    id: Uuid,
    inner: Sender<T>,
}

impl<T> PartialEq for OneshotMpscSenderHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> OneshotMpscSenderHandle<T> {
    pub fn new(sender: Sender<T>) -> Self {
        Self {
            id: Uuid::new_v4(),
            inner: sender,
        }
    }
}

#[async_trait]
impl<T> InitReplier<T> for OneshotMpscSenderHandle<T>
where
    T: Send,
{
    async fn reply_to_init_with(&self, value: T, sent_in: &str) {
        match self.send(value).await {
            Ok(_) => {
                trace!("Init signal sent in {}.", sent_in);
            }
            Err(_) => {
                error!("Error while sending init signal in {}.", sent_in);
            }
        }
    }

    async fn send(&self, value: T) -> Result<(), ()> {
        if let Err(e) = self.inner.send(value).await.map_err(|_| ()) {
            error!("Error while sending message: {:?}", e);
            Err(())
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug)]
pub struct OneshotMpscReceiverHandle<T> {
    id: Uuid,
    inner: Arc<Mutex<Receiver<T>>>,
}

impl<T> PartialEq for OneshotMpscReceiverHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> OneshotMpscReceiverHandle<T> {
    pub fn new(receiver: Receiver<T>) -> Self {
        Self {
            id: Uuid::new_v4(),
            inner: Arc::new(Mutex::new(receiver)),
        }
    }

    #[cfg(test)]
    pub async fn len(&self) -> usize {
        self.inner.lock().await.len()
    }

    pub async fn recv(&mut self) -> Option<T> {
        self.inner.lock().await.recv().await
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn send_and_recv() {
        let (sender, mut receiver) = one_shot::<bool>();

        let _ = sender.send(true).await.unwrap();
        let value = receiver.recv().await.unwrap();

        assert_eq!(value, true);
    }
}
