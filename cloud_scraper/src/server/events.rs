use crate::domain::channel_handle::ChannelHandle;
use crate::domain::mpsc_handle::OneshotMpscSenderHandle;
use derive_getters::Getters;
use oauth2::{AuthorizationCode, CsrfToken};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub enum Event {
    Redirect(String, OneshotMpscSenderHandle<String>),
    Oauth2Code(Code, String),
}

#[derive(Clone, Debug, Deserialize, Getters, Serialize)]
pub struct Code {
    code: AuthorizationCode,
    state: CsrfToken,
}

pub type WebEventChannelHandle = ChannelHandle<Event>;
