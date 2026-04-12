use serde::{Deserialize, Serialize};

use crate::config_type::{ConfigType, SelectorSideEffect};

pub const THEME_NAMES: &[&str] = &[
    "dark",
    "light",
    "gruvbox dark",
    "nord",
    "solarized",
    "tokyo night",
    "pitch",
];

pub const LOG_LEVEL_OPTIONS: &[&str] = &["error", "warn", "info", "debug", "trace"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ui {
    pub hide_bare_ips: bool,
    pub log_limit: u32,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_log_level() -> String {
    mds_default::UI_LOG_LEVEL.value.to_owned()
}

fn default_theme() -> String {
    mds_default::UI_THEME.value.to_owned()
}

impl Ui {
    pub fn items(&mut self) -> Vec<ConfigType<'_>> {
        vec![
            ConfigType::Toggle {
                key: "Hide Bare IPs",
                val: &mut self.hide_bare_ips,
                description: mds_default::UI_HIDE_BARE_IPS.description,
            },
            ConfigType::Numberu32 {
                key: "Log Limit",
                val: &mut self.log_limit,
                description: mds_default::UI_LOG_LIMIT.description,
            },
            ConfigType::StringSelect {
                key: "Log Level",
                val: &mut self.log_level,
                options: LOG_LEVEL_OPTIONS,
                description: mds_default::UI_LOG_LEVEL.description,
                side_effect: SelectorSideEffect::None,
            },
            ConfigType::StringSelect {
                key: "Theme",
                val: &mut self.theme,
                options: THEME_NAMES,
                description: mds_default::UI_THEME.description,
                side_effect: SelectorSideEffect::BumpThemeVersion,
            },
        ]
    }
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            hide_bare_ips: mds_default::UI_HIDE_BARE_IPS.value,
            log_limit: mds_default::UI_LOG_LIMIT.value,
            log_level: mds_default::UI_LOG_LEVEL.value.to_owned(),
            theme: mds_default::UI_THEME.value.to_owned(),
        }
    }
}
