use std::net::{Ipv4Addr, SocketAddrV4};

pub(crate) const MULTICAST_PORT: u16 = 5353;
pub(crate) const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
pub(crate) const MDNS_SOCKET_ADDR: SocketAddrV4 = SocketAddrV4::new(MULTICAST_ADDR, MULTICAST_PORT);
