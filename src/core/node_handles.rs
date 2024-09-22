use crate::domain::node::Manager;
use crate::server::WebEventChannelHandle;
use derive_getters::Getters;

#[derive(Clone, Debug, Getters)]
pub(crate) struct NodeHandles {
    lifecycle_manager: Manager,
    web_channel_handle: WebEventChannelHandle,
}

impl NodeHandles {
    pub(crate) fn new(
        lifecycle_manager: &Manager,
        web_event_channel_handle: &WebEventChannelHandle,
    ) -> Self {
        Self {
            lifecycle_manager: lifecycle_manager.clone(),
            web_channel_handle: web_event_channel_handle.clone(),
        }
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use crate::domain::config::tests::test_config;
    use crate::domain::node::get_test_manager;

    pub(crate) fn get_test_node_handles() -> NodeHandles {
        let config = test_config();
        let manager = get_test_manager(&config);
        let web_channel_handle = WebEventChannelHandle::new();
        NodeHandles::new(&manager, &web_channel_handle)
    }
}
