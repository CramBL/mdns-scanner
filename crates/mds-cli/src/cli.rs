use std::num::NonZeroU16;

use clap::{
    Parser,
    builder::{
        Styles,
        styling::{AnsiColor, Effects},
    },
};
use mds_util::host_up::TimeoutSettings;
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
#[derive(Parser, Debug)]
#[command(name = "MDNS Scanner", version, styles = STYLES)]
#[command(author = env!("CARGO_PKG_AUTHORS"))]
#[command(about = ABOUT)]
#[command(long_about = LONG_ABOUT)]
#[command(
    help_template = "{name} {version}\n{author-with-newline}{about-section} \n {all-args} {tab}"
)]
pub struct Args {
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

    /// Compact view (hide help footer)
    #[arg(short = 'c', long, default_value_t = false)]
    compact: bool,

    /// How long to wait before timing out a TCP connection on each individual port
    ///
    /// e.g. if the timeout is 100ms and the TCP connection is attempted on 3 different ports,
    /// each port is tried with a timeout of 100ms, for a total timeout of 300ms.
    #[arg(long = "tcp-port-timeout-ms", default_value_t = NonZeroU16::new(100).unwrap())]
    tcp_port_timeout_ms: NonZeroU16,

    /// How long to wait for echo replies
    #[arg(long = "ping-timeout-ms", default_value_t = NonZeroU16::new(300).unwrap())]
    ping_timeout_ms: NonZeroU16,

    /// Upper time limit for checking if a host is up on an IP
    #[arg(short = 'W', long = "ip-check-timeout-ms", default_value_t = NonZeroU16::new(5000).unwrap())]
    ip_check_timeout_ms: NonZeroU16,
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

    pub fn compact(&self) -> bool {
        self.compact
    }

    pub fn tcp_port_timeout_ms(&self) -> NonZeroU16 {
        self.tcp_port_timeout_ms
    }

    pub fn ping_timeout_ms(&self) -> NonZeroU16 {
        self.ping_timeout_ms
    }

    pub fn ip_check_timeout_ms(&self) -> NonZeroU16 {
        self.ip_check_timeout_ms
    }

    pub fn timeout_settings(&self) -> TimeoutSettings {
        TimeoutSettings {
            tcp_port_timeout_ms: self.tcp_port_timeout_ms,
            ping_timeout_ms: self.ping_timeout_ms,
            ip_check_timeout_ms: self.ip_check_timeout_ms,
        }
    }
}

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());
