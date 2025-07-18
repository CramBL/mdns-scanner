use ratatui::{
    style::{Color, Style},
    text::Line,
    widgets::ListItem,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    num::{NonZeroU16, NonZeroUsize},
    path::PathBuf,
};

use crate::{interfaces::Interfaces, scan::Scan, timeouts::Timeouts, ui::Ui};

mod default;
pub mod error;
pub mod interfaces;
pub mod load;
pub mod modify;
pub mod scan;
pub mod timeouts;
pub mod toggle;
pub mod ui;

#[derive(Debug)]
pub enum ConfigType<'c> {
    Toggle {
        key: &'static str,
        val: &'c mut bool,
        description: &'static str,
    },
    NumberNonZeroU16 {
        key: &'static str,
        val: &'c mut NonZeroU16,
        description: &'static str,
    },
    Numberu32 {
        key: &'static str,
        val: &'c mut u32,
        description: &'static str,
    },
    NumberList {
        key: &'static str,
        val: &'c mut Option<Vec<u16>>,
        description: &'static str,
    },
    StringList {
        key: &'static str,
        val: &'c mut Vec<String>,
        description: &'static str,
    },
}

const KEY_STR_LEN: usize = 25;

impl ConfigType<'_> {
    pub fn value_str(&self) -> String {
        match self {
            ConfigType::Toggle { val, .. } => {
                if **val {
                    "[*]".to_owned()
                } else {
                    "[ ]".to_owned()
                }
            }
            ConfigType::NumberNonZeroU16 { val, .. } => val.get().to_string(),
            ConfigType::Numberu32 { val, .. } => val.to_string(),
            ConfigType::NumberList { val, .. } => val
                .iter()
                .flatten()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            ConfigType::StringList { val, .. } => val
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
        }
    }
}

impl From<ConfigType<'_>> for ListItem<'_> {
    fn from(cfg_ty: ConfigType) -> Self {
        match cfg_ty {
            ConfigType::Toggle { key, val, .. } => {
                let checkbox = if *val { "[*]" } else { "[ ]" };

                let line = Line::styled(
                    format!("{key:<KEY_STR_LEN$}{checkbox}"),
                    if *val {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::White)
                    },
                );

                ListItem::new(line)
            }
            ConfigType::NumberNonZeroU16 { key, val, .. } => {
                let formatted_val = format!("{:>4}", val.get()); // Right-align within 4 spaces
                let value = format!("{key:<KEY_STR_LEN$}{formatted_val}");
                ListItem::new(value)
            }
            ConfigType::Numberu32 { key, val, .. } => {
                ListItem::new(format!("{key:<KEY_STR_LEN$}{val}"))
            }
            ConfigType::NumberList { key, val, .. } => {
                let mut value = format!("{key:<KEY_STR_LEN$}");

                if let Some(vals) = val {
                    if vals.is_empty() {
                        value.push('-');
                    } else {
                        let joined = vals
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", ");
                        value.push_str(&joined);
                    }
                }
                ListItem::new(value)
            }
            ConfigType::StringList { key, val, .. } => {
                let mut value = format!("{key:<KEY_STR_LEN$}");

                if val.is_empty() {
                    value.push('-');
                } else {
                    let joined = val
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ");
                    value.push_str(&joined);
                }
                ListItem::new(value)
            }
        }
    }
}

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
