use crate::domain::channel_handle::ChannelHandle;
use crate::domain::node::Manager;
use crate::integration::google::Events;
use crate::server::WebEventChannelHandle;
use derive_getters::Getters;

#[derive(Clone, Debug, Getters)]
pub struct NodeHandles {
    google_control_events: ChannelHandle<Events>,
    lifecycle_manager: Manager,
    web_channel_handle: WebEventChannelHandle,
}

impl NodeHandles {
    pub fn new(
        lifecycle_manager: &Manager,
        google_control_events: &ChannelHandle<Events>,
        web_event_channel_handle: &WebEventChannelHandle,
    ) -> Self {
        Self {
            google_control_events: google_control_events.clone(),
            lifecycle_manager: lifecycle_manager.clone(),
            web_channel_handle: web_event_channel_handle.clone(),
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::domain::config::tests::test_config;
    use crate::domain::node::get_test_manager;

    pub fn get_test_node_handles() -> NodeHandles {
        let config = test_config();
        let manager = get_test_manager(&config);
        let google_control_events = ChannelHandle::new();
        let web_channel_handle = WebEventChannelHandle::new();
        NodeHandles::new(&manager, &google_control_events, &web_channel_handle)
    }
}
