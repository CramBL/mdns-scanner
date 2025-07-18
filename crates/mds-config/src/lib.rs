use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{num::NonZeroUsize, path::PathBuf};

use crate::{interfaces::Interfaces, scan::Scan, timeouts::Timeouts, ui::Ui};

pub mod config_type;
mod default;
pub mod error;
pub mod interfaces;
pub mod load;
pub mod modify;
pub mod scan;
pub mod timeouts;
pub mod toggle;
pub mod ui;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AppConfig {
    pub scan: Scan,
    pub ui: Ui,
    pub interfaces: Interfaces,
    pub timeouts: Timeouts,
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
        self.ui.compact
    }

    pub fn timeout_settings(&self) -> Timeouts {
        self.timeouts
    }

    pub fn log_limit(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.ui.log_limit.max(1) as usize).unwrap()
    }

    /// Get service discovery enabled (inverted from CLI's no_service_discovery)
    pub fn service_discovery_enabled(&self) -> bool {
        self.scan.service_discovery
    }

    /// Get the user config file path
    pub fn user_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("mdns-scanner").join("config.toml"))
    }

    pub fn hide_bare_ips(&self) -> bool {
        self.ui.hide_bare_ips
    }

    pub fn scan_tcp_ports(&self) -> Vec<u16> {
        self.scan.tcp_ports.as_ref().map_or_else(
            || mds_default::SCAN_TCP_PORTS.value.to_vec(),
            |p| p.to_owned(),
        )
    }
}
