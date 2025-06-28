use anyhow::bail;
use dns_parser::{Builder, Packet, QueryClass, QueryType};
use std::net::UdpSocket;

use mds_log::prelude::*;
use mds_util::prelude::*;

use crate::service_registry::ServiceRegistry;

pub(crate) fn send_mdns_query(
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

pub(crate) fn build_dns_sd_query_all() -> anyhow::Result<Vec<u8>> {
    let mut builder = Builder::new_query(MDNS_QUERY_ID, false);
    builder.add_question(DNS_SD_QUERY_ALL, false, QueryType::PTR, QueryClass::IN);
    let Ok(query) = builder.build() else {
        bail!("Failed building query");
    };
    Ok(query)
}

pub(crate) fn build_query_ptr(service_type: &str) -> anyhow::Result<Vec<u8>> {
    let mut builder = Builder::new_query(MDNS_QUERY_ID, false);
    builder.add_question(service_type, false, QueryType::PTR, QueryClass::IN);
    let Ok(query) = builder.build() else {
        bail!("Failed building query");
    };
    Ok(query)
}

pub(crate) fn query_ptr(service_type: &str, socket: &UdpSocket) -> anyhow::Result<()> {
    let query = build_query_ptr(service_type)?;
    socket.send_to(&query, MDNS_SOCKET_ADDR)?;
    Ok(())
}

pub(crate) fn build_query_srv_and_txt(instance: &str) -> anyhow::Result<Vec<u8>> {
    let mut builder = Builder::new_query(MDNS_QUERY_ID, false);
    builder.add_question(instance, false, QueryType::SRV, QueryClass::IN);
    builder.add_question(instance, false, QueryType::TXT, QueryClass::IN);
    let Ok(query) = builder.build() else {
        bail!("Failed building query");
    };
    Ok(query)
}

pub(crate) fn query_srv_and_txt(instance: &str, socket: &UdpSocket) -> anyhow::Result<()> {
    let query = build_query_srv_and_txt(instance)?;
    socket.send_to(&query, MDNS_SOCKET_ADDR)?;
    Ok(())
}

pub(crate) fn build_query_a_and_aaaa(hostname: &str) -> anyhow::Result<Vec<u8>> {
    let mut builder = Builder::new_query(MDNS_QUERY_ID, false);
    builder.add_question(hostname, false, QueryType::A, QueryClass::IN);
    builder.add_question(hostname, false, QueryType::AAAA, QueryClass::IN);

    let Ok(query) = builder.build() else {
        bail!("Failed building query");
    };
    Ok(query)
}

pub(crate) fn query_a_and_aaaa(hostname: &str, socket: &UdpSocket) -> anyhow::Result<()> {
    let query = build_query_a_and_aaaa(hostname)?;
    socket.send_to(&query, MDNS_SOCKET_ADDR)?;
    Ok(())
}
