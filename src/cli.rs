use clap::{
    Parser,
    builder::{
        Styles,
        styling::{AnsiColor, Effects},
    },
};
use regex::Regex;

const ABOUT: &str = concat!(
    "Scan a network and create a list of IPs and associated hostnames",
    "\n\nProject homepage: ",
    env!("CARGO_PKG_HOMEPAGE")
);
const LONG_ABOUT: &str = concat!(
    env!("CARGO_PKG_DESCRIPTION"),
    "\n\nProject homepage: ",
    env!("CARGO_PKG_HOMEPAGE")
);

/// Network scanner application
#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"), version, styles = cli_styles())]
#[command(author = env!("CARGO_PKG_AUTHORS"))]
#[command(about = ABOUT)]
#[command(long_about = LONG_ABOUT)]
#[command(
    help_template = "{name} {version}\n{author-with-newline}{about-section} \n {usage-heading} {usage} \n {all-args} {tab}"
)]
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

    /// Don't attempt to discover DNS-SD instances
    #[arg(long = "no-dns-sd", default_value_t = false)]
    no_service_discovery: bool,
}

impl Args {
    pub fn iface_ignore_re(&self) -> Vec<Regex> {
        self.iface_ignore_re.clone()
    }

    pub fn iface_include_docker(&self) -> bool {
        self.iface_include_docker
    }

    pub fn service_discovery_enabled(&self) -> bool {
        !self.no_service_discovery
    }
}

pub fn cli_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .usage(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .literal(AnsiColor::Blue.on_default())
        .placeholder(AnsiColor::Green.on_default())
}
