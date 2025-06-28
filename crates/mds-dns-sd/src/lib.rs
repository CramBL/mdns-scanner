use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddrV4, UdpSocket},
    thread::JoinHandle,
};

use mds_log::prelude::*;
use mds_util::prelude::MULTICAST_PORT;
use socket2::{Domain, Protocol, Socket, Type};

pub mod discover;
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

pub fn spawn_dns_sd_discoverer(
    log: Logger,
) -> io::Result<JoinHandle<anyhow::Result<Vec<ServiceInfo>>>> {
    std::thread::Builder::new()
        .name("dns_sd_discoverer".into())
        .spawn(move || discover::send_dns_sd_queries(&log))
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc, time::Duration};

    use super::*;

    #[ignore = "Can take a long time, since it runs until all discovered services have all the info resolved"]
    #[test]
    fn test_handle_mdns_response_ptr() {
        let (tx_logs, rx_logs) = mpsc::channel();
        let logger = Logger::new(tx_logs, LogLevel::default());
        let h = spawn_dns_sd_discoverer(logger.clone()).unwrap();
        while let Ok(msg) = rx_logs.recv_timeout(Duration::from_secs(2)) {
            println!("{msg:?}");
        }

        let _services = h.join().unwrap().unwrap();

        while let Ok(msg) = rx_logs.recv_timeout(Duration::from_secs(2)) {
            println!("{msg:?}");
        }
    }
}
