pub mod auth;
mod source;

pub use source::Source;

#[derive(Clone, Debug)]
pub enum Events {
    OauthRedirectUrl(String),
}
