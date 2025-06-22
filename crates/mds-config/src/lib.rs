use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{interfaces::Interfaces, timeouts::Timeouts};

mod default;
pub mod error;
pub mod interfaces;
pub mod load;
pub mod modify;
pub mod timeouts;
pub mod toggle;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    service_discovery: bool,
    compact: bool,
    hide_bare_ips: bool,
    interfaces: Interfaces,
    timeouts: Timeouts,
}

impl AppConfig {
    /// Panics if called before config is loaded and regexes are compiled.
    pub fn iface_ignore_regex(&self) -> &[Regex] {
        self.interfaces.ignore_patterns()
    }

    pub fn iface_include_docker(&self) -> bool {
        self.interfaces.include_docker()
    }

    pub fn compact(&self) -> bool {
        self.compact
    }

    pub fn timeout_settings(&self) -> Timeouts {
        self.timeouts
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
