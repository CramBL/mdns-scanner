use std::{
    io,
    net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, UdpSocket},
    thread::JoinHandle,
};

use mds_util::prelude::MULTICAST_PORT;
use socket2::{Domain, Protocol, Socket, Type};

pub(crate) mod bivec;
pub mod discover;
pub mod lookup;
pub mod prelude;
mod service_registry;
pub(crate) mod util;

#[derive(Debug, PartialEq)]
pub struct ServiceInfo {
    pub name: String,
    pub _type: String,
    pub txt: Option<Vec<String>>,
    pub host: String,
    pub ipv4: Option<Ipv4Addr>,
    pub ipv6: Option<Ipv6Addr>,
    pub port: u16,
}

pub(crate) fn setup_socket() -> io::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    socket.set_nonblocking(false)?;
    let bind_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, MULTICAST_PORT);
    socket.bind(&bind_addr.into())?;
    let udp_socket: UdpSocket = socket.into();
    udp_socket.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
    Ok(udp_socket)
}

pub fn spawn_dns_sd_discoverer() -> io::Result<JoinHandle<io::Result<Vec<ServiceInfo>>>> {
    std::thread::Builder::new()
        .name("dns_sd_discoverer".into())
        .spawn(discover::send_dns_sd_queries)
}
