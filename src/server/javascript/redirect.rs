use crate::core::node_handles::NodeHandles;
use handlebars::Handlebars;
use lazy_static::lazy_static;
use std::collections::HashMap;

const JS_TEMPLATE_NAME: &str = "js";
const INLINE_TAGS: &str = "<script>
{{{script}}}
 createWebSocketConnection(\"{{{url}}}\");
</script>";

lazy_static! {
    pub static ref SCRIPT_TEMPLATE: Handlebars<'static> = {
        let mut handlebars = Handlebars::new();
        handlebars
            .register_template_string(JS_TEMPLATE_NAME, INLINE_TAGS)
            .expect("Could not register Google config template");
        handlebars
    };
}

fn redirect(domain: &str, port: u16) -> String {
    let mut map = HashMap::new();

    map.insert(
        "script",
        include_str!("../../../resources/js/redirect.js").to_string(),
    );
    map.insert("url", format!("ws://{}:{}/ws", domain, port).to_string());

    SCRIPT_TEMPLATE
        .render(JS_TEMPLATE_NAME, &map)
        .expect("Could not render script template")
}

pub trait WithRedirect {
    fn with_redirect_script(self, handles: &NodeHandles) -> Self;
}

impl WithRedirect for HashMap<&str, String> {
    fn with_redirect_script(mut self, handles: &NodeHandles) -> Self {
        let core_config = handles.lifecycle_manager().core_config();
        self.insert(
            "redirect_script",
            redirect(core_config.domain_name(), core_config.port()),
        );
        self
    }
}
