use std::net::{Ipv4Addr, SocketAddrV4};

pub(crate) const MULTICAST_PORT: u16 = 5353;
pub(crate) const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
/// In multicast query messages, the Query Identifier SHOULD be set to zero on transmission
/// https://www.rfc-editor.org/rfc/rfc6762.html
pub(crate) const MDNS_QUERY_ID: u16 = 0;
pub(crate) const MDNS_SOCKET_ADDR: SocketAddrV4 = SocketAddrV4::new(MULTICAST_ADDR, MULTICAST_PORT);
