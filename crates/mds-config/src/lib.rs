use mds_util::host_up::TimeoutSettings;
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

const DEFAULT_CONFIG: &str = include_str!("../../../docs/default_config.toml");
const DEFAULT_CONFIG_HEADER: &str = "\
# mdns-scanner configuration file
#
# Possible Locations:
";
const CONFIG_LOC_DESCRIPTION: &str = if cfg!(target_os = "windows") {
    r"
# - %APPDATA%\mdns-scanner\config.toml    (user-level, persisted)
# - .\mdns-scanner.toml                   (directory local)"
} else if cfg!(target_os = "macos") {
    "\
# - ~/Library/Application Support/mdns-scanner/config.toml    (user-level, persisted)
# - ./mdns-scanner.toml                                       (directory local)"
} else {
    "\
# - ~/.config/mdns-scanner/config.toml    (user-level, persisted)
# - ./mdns-scanner.toml                   (directory local)"
};

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

    pub fn timeout_settings(&self) -> TimeoutSettings {
        TimeoutSettings {
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

    /// Return the default config as a string
    pub fn default_config() -> String {
        let mut config = String::with_capacity(
            DEFAULT_CONFIG.len() + DEFAULT_CONFIG_HEADER.len() + CONFIG_LOC_DESCRIPTION.len(),
        );

        config.push_str(DEFAULT_CONFIG_HEADER);
        config.push_str(CONFIG_LOC_DESCRIPTION);
        config.push('\n');
        config.push('\n');

        let mut start_including = false;
        for l in DEFAULT_CONFIG.lines() {
            if !start_including && l.starts_with("#") {
                // ignore lines until the first non-commented lines
                // by convention, the default header ends with an empty non-commented line
            } else if start_including {
                config.push_str(l);
                config.push('\n');
            } else {
                start_including = true;
            }
        }
        config.push('\n');
        config
    }

    pub fn hide_bare_ips(&self) -> bool {
        self.hide_bare_ips
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_default_config_str() {
        let default = AppConfig::default_config();
        assert!(default.starts_with("#"));
    }
}
