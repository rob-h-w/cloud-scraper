use crate::domain::channel_handle::ChannelHandle;
use crate::domain::node::Manager;
use crate::integration::google::Events;
use derive_getters::Getters;

#[derive(Clone, Debug, Getters)]
pub struct NodeHandles {
    google_control_events: ChannelHandle<Events>,
    lifecycle_manager: Manager,
}

impl NodeHandles {
    pub fn new(lifecycle_manager: &Manager, google_control_events: &ChannelHandle<Events>) -> Self {
        Self {
            google_control_events: google_control_events.clone(),
            lifecycle_manager: lifecycle_manager.clone(),
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::domain::node::test::get_test_manager;

    pub fn get_test_node_handles() -> NodeHandles {
        let manager = get_test_manager();
        let google_control_events = ChannelHandle::new();
        NodeHandles::new(&manager, &google_control_events)
    }
}
