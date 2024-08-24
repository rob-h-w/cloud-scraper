use crate::domain::channel_handle::ChannelHandle;
use crate::domain::mpsc_handle::OneshotMpscSenderHandle;

#[derive(Clone, Debug)]
pub enum Event {
    Redirect(String, OneshotMpscSenderHandle<String>),
}

pub type WebEventChannelHandle = ChannelHandle<Event>;
