use std::net::{Ipv4Addr, SocketAddrV4};

pub const MULTICAST_PORT: u16 = 5353;
pub const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
/// In multicast query messages, the Query Identifier SHOULD be set to zero on transmission
/// https://www.rfc-editor.org/rfc/rfc6762.html
pub const MDNS_QUERY_ID: u16 = 0;
pub const MDNS_SOCKET_ADDR: SocketAddrV4 = SocketAddrV4::new(MULTICAST_ADDR, MULTICAST_PORT);

pub const DNS_SD_QUERY_ALL: &str = "_services._dns-sd._udp.local";
pub const DNS_SD_QUERY_ALL_NEW: &str = "_services._dns-sd._udp.local.";

/// 192.0.2.1 is from the TEST-NET-1 range reserved for documentation (RFC 5737).
/// It should never be reachable, making it suitable for testing failure cases.
/// This test validates that the function correctly times out and returns false.
pub const IP_TEST_NET_1_UNREACHABLE: Ipv4Addr = Ipv4Addr::new(192, 0, 2, 1);
