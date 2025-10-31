use serde::{Deserialize, Serialize};

use crate::config_type::ConfigType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ui {
    pub compact: bool,
    pub hide_bare_ips: bool,
    pub log_limit: u32,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_log_level() -> String {
    "info".to_owned()
}

impl Ui {
    pub fn items(&mut self) -> Vec<ConfigType<'_>> {
        vec![
            ConfigType::Toggle {
                key: "Compact Mode",
                val: &mut self.compact,
                description: mds_default::UI_COMPACT.description,
            },
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
            ConfigType::LogLevelString {
                key: "Log Level",
                val: &mut self.log_level,
                description: mds_default::UI_LOG_LEVEL.description,
            },
        ]
    }
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            compact: mds_default::UI_COMPACT.value,
            hide_bare_ips: mds_default::UI_HIDE_BARE_IPS.value,
            log_limit: mds_default::UI_LOG_LIMIT.value,
            log_level: mds_default::UI_LOG_LEVEL.value.to_owned(),
        }
    }
}
