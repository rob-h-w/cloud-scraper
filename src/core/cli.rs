use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, about)]
pub(crate) struct Cli {
    /// Config file
    #[arg(short, long)]
    pub(crate) config: Option<String>,
    /// Exit after the specified number of seconds
    #[arg(long)]
    pub(crate) exit_after: Option<u64>,
}
