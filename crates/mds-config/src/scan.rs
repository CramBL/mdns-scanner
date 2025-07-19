use serde::{Deserialize, Serialize};

use crate::config_type::ConfigType;
use io_threads::default_io_threads;

pub mod io_threads;
pub use io_threads::IoThreads;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Scan {
    pub service_discovery: bool,
    pub tcp_ports: Option<Vec<u16>>,
    #[serde(default = "default_io_threads")]
    pub io_threads: IoThreads,
}

impl Default for Scan {
    fn default() -> Self {
        Self {
            service_discovery: mds_default::SCAN_SERVICE_DISCOVERY.value,
            tcp_ports: Some(mds_default::SCAN_TCP_PORTS.value.to_vec()),
            io_threads: IoThreads::Dynamic,
        }
    }
}

impl Scan {
    pub fn items(&mut self) -> Vec<ConfigType> {
        vec![
            ConfigType::Toggle {
                key: "Service Discovery",
                val: &mut self.service_discovery,
                description: mds_default::SCAN_SERVICE_DISCOVERY.description,
            },
            ConfigType::NumberList {
                key: "TCP Ports",
                val: &mut self.tcp_ports,
                description: mds_default::SCAN_TCP_PORTS.description,
            },
            ConfigType::ScanIoThreads {
                key: "I/O Threads",
                val: &mut self.io_threads,
                description: mds_default::SCAN_IO_THREADS.description,
            },
        ]
    }
}
