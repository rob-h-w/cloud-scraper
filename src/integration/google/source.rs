use crate::domain::channel_handle::ChannelHandle;
use crate::domain::node::Manager;
use crate::integration::google::Events;
use derive_getters::Getters;

#[derive(Clone, Debug, Getters)]
pub struct Source {
    control_events: ChannelHandle<Events>,
    lifecycle_manager: Manager,
}

impl Source {
    pub fn new(manager: &Manager) -> Self {
        Self {
            control_events: ChannelHandle::new(),
            lifecycle_manager: manager.clone(),
        }
    }

    pub async fn run(&self) {}
}
