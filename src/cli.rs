use clap::{
    Parser,
    builder::{
        Styles,
        styling::{AnsiColor, Effects},
    },
};
use regex::Regex;

/// Network scanner application
#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"), version, styles = cli_styles())]
#[command(about = "Scans network interfaces and IPs")]
pub(crate) struct Args {
    /// Regex pattern to ignore network interfaces (can be used multiple times)
    #[arg(short = 'x', long = "ignore-re-iface")]
    ignore_re_iface: Vec<Regex>,
}

impl Args {
    pub fn ignore_re_iface(&self) -> Vec<Regex> {
        self.ignore_re_iface.clone()
    }
}

pub fn cli_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .usage(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .literal(AnsiColor::Blue.on_default())
        .placeholder(AnsiColor::Green.on_default())
}
