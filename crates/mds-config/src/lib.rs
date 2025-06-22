use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::timeouts::Timeouts;

mod default;
mod eq;
pub mod error;
pub mod load;
pub mod modify;
pub mod timeouts;
pub mod toggle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    iface_ignore_re: Vec<String>,
    iface_include_docker: bool,
    service_discovery: bool,
    compact: bool,
    hide_bare_ips: bool,
    timeouts: Timeouts,
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

    pub fn iface_include_docker(&self) -> bool {
        self.iface_include_docker
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
