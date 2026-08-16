use serde::{Deserialize, Serialize};

use crate::config_type::ConfigType;
use io_threads::{
    IoThreadsField, default_io_threads, deserialize_port_scan_io_threads,
    deserialize_subnet_io_threads,
};

pub mod io_threads;
pub use io_threads::IoThreads;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Scan {
    pub service_discovery: bool,
    pub tcp_ports: Option<Vec<u16>>,
    #[serde(
        default = "default_io_threads",
        deserialize_with = "deserialize_subnet_io_threads"
    )]
    pub io_threads: IoThreads,
    #[serde(
        default = "default_io_threads",
        deserialize_with = "deserialize_port_scan_io_threads"
    )]
    pub port_scan_io_threads: IoThreads,
}

impl Default for Scan {
    fn default() -> Self {
        Self {
            service_discovery: mds_default::SCAN_SERVICE_DISCOVERY.value,
            tcp_ports: Some(mds_default::SCAN_TCP_PORTS.value.to_vec()),
            io_threads: IoThreads::Dynamic,
            port_scan_io_threads: IoThreads::Dynamic,
        }
    }
}

impl Scan {
    pub fn items(&mut self) -> Vec<ConfigType<'_>> {
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
                field: IoThreadsField::Subnet,
                description: mds_default::SCAN_IO_THREADS.description,
            },
            ConfigType::ScanIoThreads {
                key: "Port Scan Threads",
                val: &mut self.port_scan_io_threads,
                field: IoThreadsField::PortScan,
                description: mds_default::SCAN_PORT_SCAN_IO_THREADS.description,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZero;

    use rstest::rstest;

    use super::*;

    fn scan_toml(io_threads: &str, port_scan_io_threads: &str) -> String {
        format!(
            "service_discovery = true\n\
             tcp_ports = [22]\n\
             io_threads = {io_threads}\n\
             port_scan_io_threads = {port_scan_io_threads}\n"
        )
    }

    fn fixed(n: u16) -> IoThreads {
        IoThreads::Fixed(NonZero::new(n).unwrap())
    }

    #[test]
    fn port_scan_io_threads_defaults_to_dynamic() {
        assert_eq!(Scan::default().port_scan_io_threads, IoThreads::Dynamic);
    }

    #[test]
    fn port_scan_io_threads_is_offered_as_its_own_config_item() {
        let mut scan = Scan::default();
        let keys: Vec<&str> = scan.items().iter().map(ConfigType::key).collect();
        assert!(keys.contains(&"I/O Threads"));
        assert!(keys.contains(&"Port Scan Threads"));
    }

    #[rstest]
    #[case("\"dynamic\"", "\"dynamic\"", IoThreads::Dynamic, IoThreads::Dynamic)]
    #[case("32", "1", fixed(32), fixed(1))]
    #[case("8192", "8192", fixed(8192), fixed(8192))]
    fn deserializes_accepted_thread_counts(
        #[case] io_threads: &str,
        #[case] port_scan_io_threads: &str,
        #[case] expected_io: IoThreads,
        #[case] expected_port_scan: IoThreads,
    ) {
        let scan: Scan = toml::from_str(&scan_toml(io_threads, port_scan_io_threads)).unwrap();
        assert_eq!(scan.io_threads, expected_io);
        assert_eq!(scan.port_scan_io_threads, expected_port_scan);
    }

    /// Only the port-scan field is varied; the subnet field stays at a fixed
    /// accepted value so a rejection can only come from the port-scan value.
    #[rstest]
    #[case("-1")]
    #[case("0")]
    #[case("99999")]
    #[case("9223372036854775807")]
    #[case("\"nope\"")]
    #[case("true")]
    #[case("1.5")]
    fn port_scan_field_rejects_out_of_range_and_wrong_typed_values(
        #[case] port_scan_io_threads: &str,
    ) {
        let toml = scan_toml("32", port_scan_io_threads);
        assert!(toml::from_str::<Scan>(&toml).is_err(), "{toml:?} parsed");
    }

    #[rstest]
    #[case("1")]
    #[case("31")]
    fn subnet_field_rejects_a_count_below_its_floor(#[case] io_threads: &str) {
        let toml = scan_toml(io_threads, "1");
        assert!(toml::from_str::<Scan>(&toml).is_err(), "{toml:?} parsed");
    }
}
