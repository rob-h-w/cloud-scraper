mod core;
pub mod domain;
pub mod integration;
mod macros;
mod main_impl;
pub mod server;
mod static_init;
mod test;

pub use main_impl::{main_impl, CoreInterface};
