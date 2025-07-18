use serde::{Deserialize, Serialize};

use crate::ConfigType;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Ui {
    pub compact: bool,
    pub hide_bare_ips: bool,
    pub log_limit: u32,
}

impl Ui {
    pub fn items(&mut self) -> Vec<ConfigType> {
        vec![
            ConfigType::Toggle {
                key: "Compact Mode",
                val: &mut self.compact,
            },
            ConfigType::Toggle {
                key: "Hide Bare IPs",
                val: &mut self.hide_bare_ips,
            },
            ConfigType::Numberu32 {
                key: "Log Limit",
                val: &mut self.log_limit,
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
        }
    }
}
