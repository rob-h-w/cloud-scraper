use crate::core::node_handles::NodeHandles;
use crate::server::auth::auth_validation;
use crate::server::javascript::WithRedirect;
use handlebars::Handlebars;
use lazy_static::lazy_static;
use std::collections::HashMap;
use warp::{reply, Filter, Rejection, Reply};

const ROOT_TEMPLATE: &str = "root";

lazy_static! {
    pub static ref PAGE_TEMPLATE: Handlebars<'static> = {
        let mut handlebars = Handlebars::new();
        handlebars
            .register_template_string(
                ROOT_TEMPLATE,
                include_str!("../../resources/html/index.html"),
            )
            .expect("Could not register login template");
        handlebars
    };
}

pub fn root(handles: &NodeHandles) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let handles = handles.clone();
    warp::path::end()
        .and(auth_validation())
        .map(move || {
            let handles = handles.clone();
            render_root(handles)
        })
        .and_then(|future| future)
}

async fn render_root(handles: NodeHandles) -> Result<impl Reply, Rejection> {
    Ok(reply::html(format_root_html(&handles)))
}

pub fn format_root_html(handles: &NodeHandles) -> String {
    let page_data = HashMap::new().with_redirect_script(handles);
    PAGE_TEMPLATE
        .render(ROOT_TEMPLATE, &page_data)
        .expect("Could not render root template")
}
