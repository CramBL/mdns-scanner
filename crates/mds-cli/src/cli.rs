use std::num::NonZeroU16;

use clap::{
    Parser, Subcommand,
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
    "Scan a network and create a list of IPs and associated hostnames, including mDNS hostnames and other aliases.",
    "\n\nProject homepage: ",
    env!("CARGO_PKG_HOMEPAGE")
);

/// Network scanner application
#[derive(Parser)]
#[command(name = "MDNS Scanner", version, styles = STYLES)]
#[command(author = env!("CARGO_PKG_AUTHORS"))]
#[command(about = ABOUT)]
#[command(long_about = LONG_ABOUT)]
#[command(
    help_template = "{name} {version}\n{author-with-newline}{about-section}\n{all-args} {tab}"
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Regex pattern(s) to ignore network interfaces (can be repeated)
    ///
    /// Example: -x 'enp5' 'eth[0-9]+$'
    #[arg(short = 'x', long = "iface-ignore-re", value_name = "PATTERNS",
    num_args = 1..,
    action = clap::ArgAction::Append)]
    pub iface_ignore_re: Vec<Regex>,

    /// Include any docker network interfaces (excluded by default)
    // Now an Option<bool> - None if not provided, Some(value) if it was
    #[arg(long = "iface-include-docker")]
    pub iface_include_docker: Option<bool>,

    /// Don't attempt to discover DNS-SD instances
    #[arg(long = "no-dns-sd")]
    pub no_service_discovery: Option<bool>,

    /// Compact view (hide help footer)
    #[arg(short = 'c', long)]
    pub compact: Option<bool>,

    /// How long to wait before timing out a TCP connection on each individual port
    ///
    /// e.g. if the timeout is 100ms and the TCP connection is attempted on 3 different ports,
    /// each port is tried with a timeout of 100ms, for a total timeout of 300ms.
    #[arg(long = "tcp-port-timeout-ms")]
    pub tcp_port_timeout_ms: Option<NonZeroU16>,

    /// How long to wait for echo replies
    #[arg(long = "ping-timeout-ms")]
    pub ping_timeout_ms: Option<NonZeroU16>,

    /// Upper time limit for checking if a host is up on an IP
    #[arg(short = 'W', long = "ip-check-timeout-ms")]
    pub ip_check_timeout_ms: Option<NonZeroU16>,
}

#[allow(missing_copy_implementations)]
#[derive(Subcommand, Clone)]
pub enum Commands {
    /// Update mdns-scanner.
    #[cfg(feature = "self-update")]
    Update(crate::self_update::SelfUpdateArgs),
    /// Write the default configuration to stdout or a file, if specified.
    DumpDefaultConfig {
        /// Path to write the default config to
        #[arg(short, long, value_name = "FILE")]
        output: Option<String>,
    },
}

impl Args {
    pub fn command(&self) -> Option<Commands> {
        self.command.clone()
    }

    pub fn iface_ignore_re(&self) -> &[Regex] {
        &self.iface_ignore_re
    }

    pub fn iface_include_docker(&self) -> bool {
        self.iface_include_docker
            .unwrap_or(mds_default::INTERFACES_INCLUDE_DOCKER.value)
    }

    pub fn service_discovery_enabled(&self) -> bool {
        self.no_service_discovery
            .map_or(mds_default::SERVICE_DISCOVERY.value, |no_sd| !no_sd)
    }

    pub fn compact(&self) -> bool {
        self.compact.unwrap_or(mds_default::UI_COMPACT.value)
    }

    pub fn tcp_port_timeout_ms(&self) -> Option<NonZeroU16> {
        self.tcp_port_timeout_ms
    }

    pub fn ping_timeout_ms(&self) -> Option<NonZeroU16> {
        self.ping_timeout_ms
    }

    pub fn ip_check_timeout_ms(&self) -> Option<NonZeroU16> {
        self.ip_check_timeout_ms
    }
}
const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());
