use mds_util::host_up::Timeouts;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU16;
use std::path::PathBuf;

mod default;
mod eq;
pub mod error;
pub mod load;
pub mod modify;
pub mod toggle;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    iface_ignore_re: Vec<String>,
    iface_include_docker: bool,
    service_discovery: bool,
    compact: bool,
    tcp_port_timeout_ms: u16,
    ping_timeout_ms: u16,
    ip_check_timeout_ms: u16,
    hide_bare_ips: bool,
    #[serde(skip)]
    compiled_iface_ignore_re: Option<Vec<Regex>>, // Cached compiled regexes
}

impl AppConfig {
    /// Get compiled regex patterns for interface ignoring from the cache.
    /// Panics if called before config is loaded and regexes are compiled.
    pub fn iface_ignore_regex(&self) -> &[Regex] {
        self.compiled_iface_ignore_re
            .as_ref()
            .expect("iface_ignore_regex called before AppConfig was fully loaded and compiled.")
    }

    /// Get TCP port timeout
    pub fn tcp_port_timeout(&self) -> Option<NonZeroU16> {
        NonZeroU16::new(self.tcp_port_timeout_ms)
    }

    /// Get ping timeout
    pub fn ping_timeout(&self) -> Option<NonZeroU16> {
        NonZeroU16::new(self.ping_timeout_ms)
    }

    /// Get IP check timeout
    pub fn ip_check_timeout(&self) -> Option<NonZeroU16> {
        NonZeroU16::new(self.ip_check_timeout_ms)
    }

    pub fn iface_include_docker(&self) -> bool {
        self.iface_include_docker
    }

    pub fn compact(&self) -> bool {
        self.compact
    }

    pub fn timeout_settings(&self) -> Timeouts {
        Timeouts {
            tcp_port_timeout_ms: self.tcp_port_timeout().unwrap(),
            ping_timeout_ms: self.ping_timeout().unwrap(),
            ip_check_timeout_ms: self.ip_check_timeout().unwrap(),
        }
    }

    /// Get service discovery enabled (inverted from CLI's no_service_discovery)
    pub fn service_discovery_enabled(&self) -> bool {
        self.service_discovery
    }

    /// Get the user config file path
    pub fn user_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("mdns-scanner").join("config.toml"))
    }

    pub fn hide_bare_ips(&self) -> bool {
        self.hide_bare_ips
    }
}
