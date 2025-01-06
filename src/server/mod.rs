mod acme;
pub(crate) mod auth;
pub(crate) mod errors;
mod events;
pub(crate) mod javascript;
mod oauth2;
mod page;
mod root;
mod routes;
mod site_state;
mod web_server;
mod websocket;

pub use events::{Code, Event, WebEventChannelHandle};

#[cfg(test)]
pub(crate) use root::format_root_html;

pub(crate) use web_server::new;
pub(crate) use web_server::WebServer;

#[cfg(test)]
pub(crate) use web_server::MockWebServer;
