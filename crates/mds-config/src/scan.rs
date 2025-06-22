use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Scan {
    pub service_discovery: bool,
}

impl Default for Scan {
    fn default() -> Self {
        Self {
            service_discovery: mds_default::SCAN_SERVICE_DISCOVERY.value,
        }
    }
}
