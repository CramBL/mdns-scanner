use dns_parser::{Packet, RData};
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4, UdpSocket};

use crate::{
    constants::{self, DNS_SD_QUERY_ALL, MDNS_SOCKET_ADDR},
    log::logger::Logger,
};
use socket2::{Domain, Protocol, Socket, Type};

use super::{ServiceInfo, service_registry::ServiceRegistry};

mod query;

pub(super) fn send_dns_sd_queries(log: &Logger) -> anyhow::Result<Vec<ServiceInfo>> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    socket.set_nonblocking(false)?;
    let bind_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, constants::MULTICAST_PORT);
    socket.bind(&bind_addr.into())?;
    let udp_socket: UdpSocket = socket.into();
    udp_socket.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;

    let query = query::build_dns_sd_query_all()?;
    udp_socket.send_to(&query, MDNS_SOCKET_ADDR)?;

    let mut registry = ServiceRegistry::new();

    query::send_mdns_query(log, &query, &udp_socket, &mut registry)?;

    let service_info = registry.finalize();
    log.info(format!("Discovered {} service(s)", service_info.len()));
    for service in &service_info {
        log.info(format!(
            "DNS-SD: {} @ {}/{}:{}",
            service.name, service.host, service.ip, service.port
        ));
    }
    Ok(service_info)
}

pub(super) fn handle_mdns_response(
    log: &Logger,
    packet: &Packet,
    socket: &UdpSocket,
    registry: &mut ServiceRegistry,
) -> anyhow::Result<()> {
    for answer in &packet.answers {
        match &answer.data {
            RData::PTR(name) => {
                let service = answer.name.to_string();
                let instance = name.to_string();
                log.trace(format!("PTR: {service} -> {instance}"));

                if service == DNS_SD_QUERY_ALL {
                    log.info(format!("Discovered service type: {instance}"));
                    query::query_ptr(log, &instance, socket, registry)?;
                } else {
                    log.info(format!("Discovered service instance: {instance}"));
                    registry.insert_or_update_instance(instance.clone(), service);
                    query::query_srv_and_txt(log, &instance, socket, registry)?;
                }
            }
            RData::SRV(srv) => {
                let instance = answer.name.to_string();
                let host = srv.target.to_string();
                let port = srv.port;
                log.debug(format!("SRV: {instance} -> {host}:{port}"));
                registry.set_srv(&instance, host.clone(), port);
                query::query_a_and_aaaa(log, &host, socket, registry)?;
            }
            RData::TXT(txt) => {
                let instance = answer.name.to_string();
                let parsed_txt = txt
                    .iter()
                    .map(|s| String::from_utf8_lossy(s).into_owned())
                    .collect::<Vec<_>>();
                log.debug(format!("TXT: {instance} -> {parsed_txt:?}"));
                registry.set_txt(&instance, parsed_txt);
            }
            RData::A(ip) => {
                let hostname = answer.name.to_string();
                let ip = ip.0;
                log.debug(format!("A: {hostname} -> {ip}"));
                registry.set_ip_for_host(&hostname, IpAddr::V4(ip));
            }
            RData::AAAA(ip6) => {
                let hostname = answer.name.to_string();
                let ip = ip6.0;
                log.debug(format!("AAAA: {hostname} -> {ip}"));
                registry.set_ip_for_host(&hostname, IpAddr::V6(ip));
            }
            _ => {}
        }
    }
    Ok(())
}
