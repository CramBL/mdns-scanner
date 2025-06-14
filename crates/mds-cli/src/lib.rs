use clap::Parser;

pub mod cli;

pub use cli::Args;

/// Parse command-line args to [Args]
pub fn parse_cli_args() -> Args {
    Args::parse()
}
