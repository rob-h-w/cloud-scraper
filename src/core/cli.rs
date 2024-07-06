use clap::{Args, Parser, Subcommand};
use serde::Deserialize;

#[derive(Debug, Parser)]
#[command(about = "Tool to set up and run a Small Technology inspired web service that integrates \
with your Big Web services on your behalf.", version = env!("CARGO_PKG_VERSION"))]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Args, Clone, Debug, Deserialize, PartialEq)]
pub struct RootPasswordArgs;

#[derive(Args, Clone, Debug, Deserialize, PartialEq)]
pub struct ServeArgs {
    /// Config file
    #[arg(short, long)]
    pub(crate) config: Option<String>,
    /// Exit after the specified number of seconds
    #[arg(long)]
    pub(crate) exit_after: Option<u64>,
    /// Port to listen on
    #[arg(short, long)]
    pub port: Option<u16>,
}

impl ServeArgs {
    pub(crate) fn default() -> ServeArgs {
        Self {
            config: None,
            exit_after: None,
            port: Some(8080),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Subcommand)]
pub enum Command {
    RootPassword(RootPasswordArgs),
    Serve(ServeArgs),
}
