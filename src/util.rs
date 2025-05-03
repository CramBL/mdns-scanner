use dns_parser::{QueryClass, QueryType};
use std::net::Ipv4Addr;

use crate::constants::MDNS_QUERY_ID;

pub(crate) fn prefix_to_netmask(prefix_len: u8) -> Ipv4Addr {
    let mask = if prefix_len == 0 {
        0
    } else {
        (!0u32) << (32 - prefix_len)
    };
    Ipv4Addr::from(mask)
}

pub(crate) fn get_network_address_from_prefix(ip: Ipv4Addr, prefix_len: u8) -> Ipv4Addr {
    let ip_u32 = u32::from(ip);

    // Create a mask from the prefix
    let mask = !0u32 << (32 - prefix_len);

    // Apply the mask and convert back
    Ipv4Addr::from(ip_u32 & mask)
}

#[derive(Debug)]
pub(crate) struct NetworkInterface {
    name: String,
    ip: Ipv4Addr,
    prefix: u8,
}

impl NetworkInterface {
    pub fn new(name: String, ip: Ipv4Addr, prefix: u8) -> Self {
        Self { name, ip, prefix }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn ip(&self) -> Ipv4Addr {
        self.ip
    }

    pub fn prefix(&self) -> u8 {
        self.prefix
    }
}

/// Determines if an interface is likely a Docker-related interface
fn is_docker_interface(name: &str) -> bool {
    // Common Docker interface patterns
    let docker_patterns = [
        // Direct docker bridge interfaces
        "docker", // Matches docker0, docker1, docker_br, etc.
        "podman",
        // Virtual Ethernet pairs used by Docker
        "veth", // Docker container connections
        "br-",  // Docker bridge networks
    ];

    for pat in docker_patterns {
        if name.starts_with(pat) {
            return true;
        }
    }

    false
}

pub(crate) fn get_network_interfaces(include_docker: bool) -> Vec<NetworkInterface> {
    let mut interfaces = pnet::datalink::interfaces();
    // Unified predicate based on filter variant
    #[cfg(unix)]
    interfaces.retain(|i| {
        let mut keep = !i.is_loopback() && i.is_up() && !i.ips.is_empty() && i.is_running();
        if include_docker {
            keep
        } else {
            keep && !is_docker_interface(&i.name)
        }
    });
    #[cfg(windows)]
    interfaces.retain(|i| {
        i.ips.iter().any(|ip| match ip {
            pnet::ipnetwork::IpNetwork::V4(ipv4_network) => !ipv4_network.ip().is_unspecified(),
            pnet::ipnetwork::IpNetwork::V6(ipv6_network) => !ipv6_network.ip().is_unspecified(),
        })
    });

    let mut net_ifs = vec![];
    for interface in interfaces {
        let pnet::datalink::NetworkInterface {
            name,
            description: _,
            index: _,
            mac: _,
            ips,
            flags: _,
        } = interface;
        for ip in ips {
            match ip {
                pnet::ipnetwork::IpNetwork::V4(ipv4_network) => {
                    let ipv4 = ipv4_network.ip();
                    let prefix = ipv4_network.prefix();
                    net_ifs.push(NetworkInterface::new(name, ipv4, prefix));
                    break;
                }
                pnet::ipnetwork::IpNetwork::V6(_) => (),
            }
        }
    }
    net_ifs
}

#[allow(dead_code, reason = "TODO: Add some clever service discovery stuff")]
pub(crate) fn build_mdns_queries() -> Vec<Vec<u8>> {
    let mut packets = Vec::new();

    let mut builder = dns_parser::Builder::new_query(MDNS_QUERY_ID, false);
    builder.add_question(
        "_services._dns-sd._udp.local",
        false,
        QueryType::PTR,
        QueryClass::IN,
    );
    packets.push(builder.build().unwrap());

    let mut builder = dns_parser::Builder::new_query(MDNS_QUERY_ID, false);
    builder.add_question(
        "_device-info._tcp.local",
        false,
        QueryType::PTR,
        QueryClass::IN,
    );
    packets.push(builder.build().unwrap());

    let mut builder = dns_parser::Builder::new_query(MDNS_QUERY_ID, false);
    builder.add_question(
        "_workstation._tcp.local",
        false,
        QueryType::PTR,
        QueryClass::IN,
    );
    packets.push(builder.build().unwrap());

    packets
}

pub(crate) fn build_reverse_dns_query(ip: Ipv4Addr) -> Vec<u8> {
    let reverse_ptr = reverse_dns_ptr_record(ip);
    let mut builder = dns_parser::Builder::new_query(MDNS_QUERY_ID, false);
    builder.add_question(&reverse_ptr, false, QueryType::PTR, QueryClass::IN);
    builder.build().unwrap()
}

#[inline]
fn reverse_dns_ptr_record(ip: Ipv4Addr) -> String {
    const ARPA_SUFFIX: &str = ".in-addr.arpa";
    let [a, b, c, d] = ip.octets();
    let mut reverse_ptr = String::with_capacity("123.123.123.123".len() + ARPA_SUFFIX.len());
    reverse_ptr.push_str(&d.to_string());
    reverse_ptr.push('.');
    reverse_ptr.push_str(&c.to_string());
    reverse_ptr.push('.');
    reverse_ptr.push_str(&b.to_string());
    reverse_ptr.push('.');
    reverse_ptr.push_str(&a.to_string());
    reverse_ptr.push_str(ARPA_SUFFIX);
    reverse_ptr
}

pub(crate) fn calc_network_host_range(prefix_len: u8) -> std::ops::Range<u32> {
    let host_bits = 32 - prefix_len;
    let host_count = 2u32.pow(host_bits as u32);
    // Skip network address (0) and broadcast address (host_count - 1)
    1..host_count - 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_reverse_dns_ptr_record() {
        let ip_str = "192.168.0.1";
        let expect_reversed_ip_str = "1.0.168.192.in-addr.arpa";
        let ip = Ipv4Addr::from_str(ip_str).unwrap();
        let reversed = reverse_dns_ptr_record(ip);
        assert_eq!(reversed, expect_reversed_ip_str);
    }

    #[test]
    fn test_get_network_address_from_prefix() {
        let ip = Ipv4Addr::new(192, 168, 1, 5);
        let prefix = 24;
        let expected_addr = Ipv4Addr::new(192, 168, 1, 0);

        let network_addr_from_prefix = get_network_address_from_prefix(ip, prefix);
        assert_eq!(expected_addr, network_addr_from_prefix);
    }

    #[test]
    fn test_get_network_interfaces() {
        let ifv = get_network_interfaces(true);
        assert!(!ifv.is_empty());
    }
}
