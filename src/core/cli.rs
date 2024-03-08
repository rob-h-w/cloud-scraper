use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, about)]
pub(crate) struct Cli {
    /// Exit after the specified number of seconds
    #[arg(long)]
    pub(crate) exit_after: Option<u64>,
}
