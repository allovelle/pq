pub use clap::Parser;

const CLI_USAGE: &str = include_str!("USAGE.txt");

/// Queries JSON from the CLI
#[derive(Parser, Debug)]
#[command(version, about, long_about = CLI_USAGE)]
pub struct Cli
{
    /// The JSON file path to ingest & query. Defaults to STDIN.
    #[arg(short, long)]
    pub file: Option<String>,

    /// The query to run on the provided JSON
    pub query: Option<String>,
}

pub fn parse() -> Cli
{
    Cli::parse()
}

pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const BLUE: &str = "\x1b[34m";
pub const YELLOW: &str = "\x1b[33m";
pub const RESET: &str = "\x1b[0m";
