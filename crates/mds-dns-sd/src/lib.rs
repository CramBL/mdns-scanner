use std::{
    io,
    net::{Ipv4Addr, SocketAddrV4, UdpSocket},
    thread::JoinHandle,
};

use mds_ipinfo::IpForHost;
use mds_util::{
    constants::MULTICAST_ADDR,
    prelude::{DnsName, MULTICAST_PORT},
};
use socket2::{Domain, Protocol, Socket, Type};

pub(crate) mod bivec;
pub mod discover;
pub mod lookup;
pub mod prelude;
mod service_registry;
pub(crate) mod util;

#[derive(Debug)]
pub struct ServiceInfo {
    pub name: String,
    pub _type: String,
    pub txt: Option<Vec<String>>,
    pub host: DnsName,
    pub ip: IpForHost,
    pub port: u16,
}

impl PartialEq for ServiceInfo {
    fn eq(&self, other: &Self) -> bool {
        let Self {
            name,
            _type,
            txt,
            host,
            ip,
            port,
        } = self;
        *name == other.name
            && *_type == other._type
            && *txt == other.txt
            && host.matches(&other.host)
            && *ip == other.ip
            && *port == other.port
    }
}

pub(crate) fn setup_socket() -> io::Result<UdpSocket> {
    setup_socket_on(MULTICAST_PORT, Ipv4Addr::UNSPECIFIED)
}

fn setup_socket_on(port: u16, interface: Ipv4Addr) -> io::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    // macOS and the BSDs need SO_REUSEPORT to share port 5353 with the system
    // mDNS responder that already holds it.
    #[cfg(unix)]
    socket.set_reuse_port(true)?;
    socket.set_nonblocking(false)?;
    let bind_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
    socket.bind(&bind_addr.into())?;
    let udp_socket: UdpSocket = socket.into();
    // RFC 6762 Section 5.2 requires a querier using source port 5353 to receive
    // responses sent to the mDNS multicast group.
    udp_socket.join_multicast_v4(&MULTICAST_ADDR, &interface)?;
    udp_socket.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
    Ok(udp_socket)
}

pub fn spawn_dns_sd_discoverer() -> io::Result<JoinHandle<io::Result<Vec<ServiceInfo>>>> {
    std::thread::Builder::new()
        .name("dns_sd_discoverer".into())
        .spawn(discover::send_dns_sd_queries)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn socket_receives_multicast_on_joined_interface() -> io::Result<()> {
        let receiver = Socket::from(setup_socket_on(0, Ipv4Addr::LOCALHOST)?);
        // Linux can otherwise deliver traffic for a group joined by another
        // local process, which would hide a missing membership on this socket.
        receiver.set_multicast_all_v4(false)?;
        let receiver: UdpSocket = receiver.into();
        receiver.set_read_timeout(Some(Duration::from_secs(1)))?;

        let sender = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        sender.set_multicast_if_v4(&Ipv4Addr::LOCALHOST)?;
        sender.set_multicast_loop_v4(true)?;
        let destination = SocketAddrV4::new(MULTICAST_ADDR, receiver.local_addr()?.port());
        let payload = b"mdns multicast membership regression";
        sender.send_to(payload, &destination.into())?;

        let mut received = [0; 64];
        let (len, _) = receiver.recv_from(&mut received)?;
        assert_eq!(&received[..len], payload);
        Ok(())
    }
}
