use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddrV4, UdpSocket},
    thread::JoinHandle,
};

use mds_util::prelude::MULTICAST_PORT;
use socket2::{Domain, Protocol, Socket, Type};

pub mod discover;
pub mod lookup;
pub mod prelude;
mod service_registry;

#[derive(Debug, PartialEq)]
pub struct ServiceInfo {
    pub name: String,
    pub _type: String,
    pub txt: Option<Vec<String>>,
    pub host: String,
    pub ip: IpAddr,
    pub port: u16,
}

pub(crate) fn setup_socket() -> anyhow::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    socket.set_nonblocking(false)?;
    let bind_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, MULTICAST_PORT);
    socket.bind(&bind_addr.into())?;
    let udp_socket: UdpSocket = socket.into();
    udp_socket.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
    Ok(udp_socket)
}

pub fn spawn_dns_sd_discoverer() -> io::Result<JoinHandle<anyhow::Result<Vec<ServiceInfo>>>> {
    std::thread::Builder::new()
        .name("dns_sd_discoverer".into())
        .spawn(discover::send_dns_sd_queries)
}
