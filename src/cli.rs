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
    /// Regex pattern(s) to ignore network interfaces (can be repeated)
    ///
    /// Example: -x 'enp5' 'eth[0-9]+$'
    #[arg(short = 'x', long = "iface-ignore-re", value_name = "PATTERNS",
    num_args = 1..,
    action = clap::ArgAction::Append)]
    iface_ignore_re: Vec<Regex>,

    /// Include any docker network interfaces (excluded by default)
    #[arg(long = "iface-include-docker", default_value_t = false)]
    iface_include_docker: bool,
}

impl Args {
    pub fn iface_ignore_re(&self) -> Vec<Regex> {
        self.iface_ignore_re.clone()
    }

    pub fn iface_include_docker(&self) -> bool {
        self.iface_include_docker
    }
}

pub fn cli_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .usage(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .literal(AnsiColor::Blue.on_default())
        .placeholder(AnsiColor::Green.on_default())
}
