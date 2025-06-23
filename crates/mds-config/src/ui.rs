use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Ui {
    pub compact: bool,
    pub hide_bare_ips: bool,
    pub log_limit: u32,
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
