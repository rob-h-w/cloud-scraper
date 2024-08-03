mod config;
pub(crate) mod google_keep;
mod hub;
pub(crate) mod source;

#[derive(Clone, Debug)]
pub enum Events {
    OauthRedirectUrl(String),
}
