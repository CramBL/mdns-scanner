pub mod constants;
pub mod debug_expect;
pub mod emojis;
pub mod host_up;
pub mod hostname;
pub mod ping;
pub mod prelude;
pub mod refresh;
pub mod resource_scaling;

use std::{net::Ipv4Addr, ops::Range};

pub fn prefix_to_netmask(prefix_len: u8) -> Ipv4Addr {
    let mask = if prefix_len == 0 {
        0
    } else {
        (!0u32) << (32 - prefix_len)
    };
    Ipv4Addr::from(mask)
}

pub fn get_network_address_from_prefix(ip: Ipv4Addr, prefix_len: u8) -> Ipv4Addr {
    let ip_u32 = u32::from(ip);
    let mask = u32::from(prefix_to_netmask(prefix_len));

    // Apply the mask and convert back
    Ipv4Addr::from(ip_u32 & mask)
}

#[derive(Debug)]
pub struct NetworkInterface {
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

    pub fn host_range(&self) -> Range<u32> {
        calc_network_host_range(self.prefix())
    }

    pub fn host_count(&self) -> u32 {
        let host_range = self.host_range();
        host_range.end - host_range.start
    }
}

/// Determines if an interface is likely a Docker-related interface
#[cfg(unix)]
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

pub fn get_network_interfaces(_include_docker: bool) -> Vec<NetworkInterface> {
    let mut interfaces = pnet::datalink::interfaces();
    // Unified predicate based on filter variant
    #[cfg(unix)]
    interfaces.retain(|i| {
        let keep = !i.is_loopback() && i.is_up() && !i.ips.is_empty() && i.is_running();
        if _include_docker {
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
                // TODO: Ipv6 network interface support
                pnet::ipnetwork::IpNetwork::V6(_) => (),
            }
        }
    }
    net_ifs
}

pub fn calc_network_host_range(prefix_len: u8) -> Range<u32> {
    match prefix_len {
        // The /0 broadcast address is u32::MAX, so the exclusive end fits in u32.
        0 => 1..u32::MAX,
        // RFC 3021 makes both addresses usable on a /31; a /32 has one address.
        31 => 0..2,
        32 => 0..1,
        1..=30 => 1..(1u32 << (32 - prefix_len)) - 1,
        _ => panic!("IPv4 prefix length must be at most 32"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn test_get_network_address_from_prefix() {
        let ip = Ipv4Addr::new(192, 168, 1, 5);
        let prefix = 24;
        let expected_addr = Ipv4Addr::new(192, 168, 1, 0);

        let network_addr_from_prefix = get_network_address_from_prefix(ip, prefix);
        assert_eq!(expected_addr, network_addr_from_prefix);
        assert_eq!(
            get_network_address_from_prefix(ip, 0),
            Ipv4Addr::UNSPECIFIED
        );
    }

    #[rstest]
    #[case(0, 1..u32::MAX, u32::MAX - 1)]
    #[case(24, 1..255, 254)]
    #[case(30, 1..3, 2)]
    #[case(31, 0..2, 2)]
    #[case(32, 0..1, 1)]
    fn host_ranges_and_counts(
        #[case] prefix: u8,
        #[case] expected_range: Range<u32>,
        #[case] expected_count: u32,
    ) {
        let network =
            NetworkInterface::new("eth0".to_owned(), Ipv4Addr::new(192, 168, 1, 10), prefix);
        assert_eq!(network.host_range(), expected_range);
        assert_eq!(network.host_count(), expected_count);
    }

    #[cfg(unix)]
    #[test]
    fn test_get_network_interfaces() {
        let ifv = get_network_interfaces(true);
        assert!(!ifv.is_empty());
    }
}
