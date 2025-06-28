use anyhow::bail;
use dns_parser::{Builder, Packet, QueryClass, QueryType};
use std::net::UdpSocket;

use mds_log::prelude::*;
use mds_util::prelude::*;

use crate::service_registry::ServiceRegistry;

pub(super) fn send_mdns_query(
    log: &Logger,
    query: &[u8],
    socket: &UdpSocket,
    registry: &mut ServiceRegistry,
) -> anyhow::Result<()> {
    socket.send_to(query, MDNS_SOCKET_ADDR)?;

    let mut buf = [0u8; 1500];
    while let Ok((len, _src)) = socket.recv_from(&mut buf) {
        match Packet::parse(&buf[..len]) {
            Ok(packet) => super::handle_mdns_response(log, &packet, socket, registry)?,
            Err(e) => log.warn(format!("mDNS packet error: {e}")),
        }
    }
    Ok(())
}

pub(super) fn build_dns_sd_query_all() -> anyhow::Result<Vec<u8>> {
    let mut builder = Builder::new_query(MDNS_QUERY_ID, false);
    builder.add_question(DNS_SD_QUERY_ALL, false, QueryType::PTR, QueryClass::IN);
    let Ok(query) = builder.build() else {
        bail!("Failed building query");
    };
    Ok(query)
}

pub(super) fn query_ptr(
    log: &Logger,
    service_type: &str,
    socket: &UdpSocket,
    registry: &mut ServiceRegistry,
) -> anyhow::Result<()> {
    let mut builder = Builder::new_query(MDNS_QUERY_ID, false);
    builder.add_question(service_type, false, QueryType::PTR, QueryClass::IN);
    let Ok(query) = builder.build() else {
        bail!("Failed building query");
    };
    send_mdns_query(log, &query, socket, registry)?;
    Ok(())
}

pub(super) fn query_srv_and_txt(
    log: &Logger,
    instance: &str,
    socket: &UdpSocket,
    registry: &mut ServiceRegistry,
) -> anyhow::Result<()> {
    let mut builder = Builder::new_query(MDNS_QUERY_ID, false);
    builder.add_question(instance, false, QueryType::SRV, QueryClass::IN);
    builder.add_question(instance, false, QueryType::TXT, QueryClass::IN);
    let Ok(query) = builder.build() else {
        bail!("Failed building query");
    };
    send_mdns_query(log, &query, socket, registry)?;
    Ok(())
}

pub(super) fn query_a_and_aaaa(
    log: &Logger,
    hostname: &str,
    socket: &UdpSocket,
    registry: &mut ServiceRegistry,
) -> anyhow::Result<()> {
    let mut builder = Builder::new_query(MDNS_QUERY_ID, false);
    builder.add_question(hostname, false, QueryType::A, QueryClass::IN);
    builder.add_question(hostname, false, QueryType::AAAA, QueryClass::IN);

    let Ok(query) = builder.build() else {
        bail!("Failed building query");
    };
    send_mdns_query(log, &query, socket, registry)?;
    Ok(())
}
