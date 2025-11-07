pub mod constants;
pub mod debug_expect;
pub mod emojis;
pub mod host_up;
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

    // Create a mask from the prefix
    let mask = !0u32 << (32 - prefix_len);

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
        (host_range.end - host_range.start).saturating_sub(1) // -1 as the range is not inclusive
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
    let host_bits = 32 - prefix_len;
    let host_count = 2u32.pow(host_bits as u32);
    // Skip network address (0) and broadcast address (host_count - 1)
    1..host_count - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_network_address_from_prefix() {
        let ip = Ipv4Addr::new(192, 168, 1, 5);
        let prefix = 24;
        let expected_addr = Ipv4Addr::new(192, 168, 1, 0);

        let network_addr_from_prefix = get_network_address_from_prefix(ip, prefix);
        assert_eq!(expected_addr, network_addr_from_prefix);
    }

    #[cfg(unix)]
    #[test]
    fn test_get_network_interfaces() {
        let ifv = get_network_interfaces(true);
        assert!(!ifv.is_empty());
    }
}
