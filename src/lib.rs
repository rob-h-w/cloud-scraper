mod core;
pub mod domain;
mod integration;
mod macros;
mod main_impl;
mod server;
mod static_init;
mod test;

pub use main_impl::{main_impl, CoreInterface};
