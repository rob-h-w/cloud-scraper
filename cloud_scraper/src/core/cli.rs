use clap::{Args, Parser, Subcommand};
use serde::Deserialize;

pub const DEFAULT_CONFIG_NAME: &str = "config.yaml";

pub trait ConfigFileProvider {
    fn config_file(&self) -> String;
}

#[derive(Debug, Parser)]
#[command(about = "Tool to set up and run a Small Technology inspired web service that integrates \
with your Big Web services on your behalf.", version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Args, Clone, Debug, Deserialize, PartialEq)]
pub struct ConfigArgs {
    /// Config file
    #[arg(short, long)]
    pub(crate) config: Option<String>,
}

impl ConfigFileProvider for ConfigArgs {
    fn config_file(&self) -> String {
        self.config
            .clone()
            .unwrap_or_else(|| DEFAULT_CONFIG_NAME.to_string())
    }
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

impl ConfigFileProvider for ServeArgs {
    fn config_file(&self) -> String {
        self.config
            .clone()
            .unwrap_or_else(|| DEFAULT_CONFIG_NAME.to_string())
    }
}

#[cfg(test)]
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
    Config(ConfigArgs),
    RootPassword(RootPasswordArgs),
    Serve(ServeArgs),
}
