use clap::{Parser, Subcommand};
use serde::Deserialize;

#[derive(Debug, Parser)]
#[command(version, about)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
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

impl Cli {
    pub fn get_command(&self) -> &Command {
        self.command.as_ref().unwrap_or(&Command::Serve)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Subcommand)]
pub enum Command {
    RootPassword,
    Serve,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_default_command() {
        let cli = Cli {
            command: None,
            config: None,
            exit_after: None,
            port: None,
        };
        assert_eq!(cli.get_command(), &Command::Serve);
    }
}
