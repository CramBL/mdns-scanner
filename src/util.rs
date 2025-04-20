use dns_parser::{QueryClass, QueryType};
use get_if_addrs::Ifv4Addr;
use std::net::Ipv4Addr;

use crate::constants::MDNS_QUERY_ID;

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
}
