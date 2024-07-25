use tokio::sync::broadcast::error::SendError;
use tokio::sync::broadcast::{channel, Receiver, Sender};

pub struct ChannelHandle<T> {
    sender: Sender<T>,
}

impl<T: Clone> ChannelHandle<T> {
    pub fn new() -> ChannelHandle<T> {
        let (sender, _receiver) = channel::<T>(12);

        ChannelHandle { sender }
    }

    pub fn get_receiver(&self) -> Receiver<T> {
        self.sender.subscribe()
    }

    pub fn send(&mut self, value: T) -> Result<usize, SendError<T>> {
        self.sender.send(value)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn send() {
        let mut handle = ChannelHandle::new();

        let _ = handle.send(true);
        let _ = handle.send(false);
    }
}
