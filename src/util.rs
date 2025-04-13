use dns_parser::{QueryClass, QueryType};
use get_if_addrs::Ifv4Addr;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

pub(crate) fn is_host_up(ip: Ipv4Addr) -> bool {
    // Try to connect to a common port (80) with a very short timeout
    let socket_addr = SocketAddr::new(IpAddr::V4(ip), 80);
    TcpStream::connect_timeout(&socket_addr, Duration::from_millis(100)).is_ok()
}

pub(crate) fn count_netmask_bits(netmask: Ipv4Addr) -> u8 {
    netmask.to_bits().count_ones() as u8
}

pub(crate) fn get_network_address(network: &Ifv4Addr) -> Ipv4Addr {
    let ip_int = u32::from(network.ip);
    let mask_int = u32::from(network.netmask);
    Ipv4Addr::from(ip_int & mask_int)
}

// Determine network parameters from available interfaces
pub(crate) fn get_network_params() -> Vec<Ifv4Addr> {
    let mut networks = Vec::new();

    if let Ok(interfaces) = get_if_addrs::get_if_addrs() {
        for iface in interfaces {
            if iface.is_loopback() {
                continue;
            }

            // Extract IP and netmask correctly
            match iface.addr {
                get_if_addrs::IfAddr::V4(ifv4_addr) => {
                    networks.push(ifv4_addr);
                }
                _ => continue, // Skip IPv6 addresses
            }
        }
    }

    networks
}

pub(crate) fn build_mdns_queries() -> Vec<Vec<u8>> {
    let mut packets = Vec::new();

    // 1. Discover available services
    let mut builder = dns_parser::Builder::new_query(1, false);
    builder.add_question(
        "_services._dns-sd._udp.local",
        false,
        QueryType::PTR,
        QueryClass::IN,
    );
    packets.push(builder.build().unwrap());

    // 3. Try an "all records" query for the `.local` domain
    let mut builder = dns_parser::Builder::new_query(2, false);
    builder.add_question(".local", true, QueryType::All, QueryClass::IN);
    packets.push(builder.build().unwrap());

    let mut builder = dns_parser::Builder::new_query(3, false);
    builder.add_question(
        "_device-info._tcp.local",
        false,
        QueryType::PTR,
        QueryClass::IN,
    );
    packets.push(builder.build().unwrap());

    // 4. Add a query for workstation information
    let mut builder = dns_parser::Builder::new_query(4, false);
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
    let octets = ip.octets();
    let reverse_ptr = format!(
        "{}.{}.{}.{}.in-addr.arpa",
        octets[3], octets[2], octets[1], octets[0]
    );

    let mut builder = dns_parser::Builder::new_query(10, false);
    builder.add_question(&reverse_ptr, false, QueryType::PTR, QueryClass::IN);
    builder.build().unwrap()
}
