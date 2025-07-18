use serde::{Deserialize, Serialize};

use crate::ConfigType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Scan {
    pub service_discovery: bool,
    pub tcp_ports: Option<Vec<u16>>,
}

impl Default for Scan {
    fn default() -> Self {
        Self {
            service_discovery: mds_default::SCAN_SERVICE_DISCOVERY.value,
            tcp_ports: Some(mds_default::SCAN_TCP_PORTS.value.to_vec()),
        }
    }
}

impl Scan {
    pub fn items(&mut self) -> Vec<ConfigType> {
        vec![
            ConfigType::Toggle {
                key: "Service Discovery",
                val: &mut self.service_discovery,
            },
            ConfigType::NumberList {
                key: "TCP Ports",
                val: &mut self.tcp_ports,
            },
        ]
    }
}
