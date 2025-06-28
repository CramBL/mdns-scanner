use crate::service_registry::ServiceRegistry;
use hickory_proto::op::{Message, Query};
use hickory_proto::rr::{Name, RecordType};
use mds_log::prelude::*;
use mds_util::prelude::*;
use std::net::UdpSocket;
use std::str::FromStr;

pub(super) fn send_mdns_query(
    log: &Logger,
    query: &[u8],
    socket: &UdpSocket,
    registry: &mut ServiceRegistry,
) -> anyhow::Result<()> {
    socket.send_to(query, MDNS_SOCKET_ADDR)?;
    let mut buf = [0u8; 1500];

    while let Ok((len, _src)) = socket.recv_from(&mut buf) {
        match super::parse_dns_response(&buf[..len]) {
            Ok(msg) => super::handle_mdns_response(log, &msg, socket, registry)?,
            Err(e) => log.warn(format!("mDNS response handling error: {e}")),
        }
    }
    Ok(())
}

#[allow(dead_code, reason = "used to confirm that the const buffer is correct")]
pub(super) fn build_dns_sd_query_all_() -> anyhow::Result<Vec<u8>> {
    build_query(DNS_SD_QUERY_ALL, &[RecordType::PTR])
}

pub(super) fn query_ptr(
    log: &Logger,
    service_type: &str,
    socket: &UdpSocket,
    registry: &mut ServiceRegistry,
) -> anyhow::Result<()> {
    let query = build_query(service_type, &[RecordType::PTR])?;
    send_mdns_query(log, &query, socket, registry)
}

pub(super) fn query_srv_and_txt(
    log: &Logger,
    instance: &str,
    socket: &UdpSocket,
    registry: &mut ServiceRegistry,
) -> anyhow::Result<()> {
    let query = build_query(instance, &[RecordType::SRV, RecordType::TXT])?;
    send_mdns_query(log, &query, socket, registry)
}

pub(super) fn query_a_and_aaaa(
    log: &Logger,
    hostname: &str,
    socket: &UdpSocket,
    registry: &mut ServiceRegistry,
) -> anyhow::Result<()> {
    let query = build_query(hostname, &[RecordType::A, RecordType::AAAA])?;
    send_mdns_query(log, &query, socket, registry)
}

fn build_query(name: &str, record_types: &[RecordType]) -> anyhow::Result<Vec<u8>> {
    let name = Name::from_str(name)?;
    let mut message = Message::new();
    message.set_id(MDNS_QUERY_ID);
    message.set_recursion_desired(false);

    for &record_type in record_types {
        let q = Query::query(name.clone(), record_type);
        message.add_query(q);
    }

    Ok(message.to_vec()?)
}

// Pre-computed const query bytes
pub(crate) const DNS_SD_QUERY_ALL_BYTES: &[u8] = &[
    0x00, 0x00, // Transaction ID (MDNS_QUERY_ID)
    0x00, 0x00, // Flags (query, not authoritative)
    0x00, 0x01, // Questions count
    0x00, 0x00, // Answer RRs count
    0x00, 0x00, // Authority RRs count
    0x00, 0x00, // Additional RRs count
    // Question: _services._dns-sd._udp.local
    0x09, b'_', b's', b'e', b'r', b'v', b'i', b'c', b'e', b's', 0x07, b'_', b'd', b'n', b's', b'-',
    b's', b'd', 0x04, b'_', b'u', b'd', b'p', 0x05, b'l', b'o', b'c', b'a', b'l',
    0x00, // End of name
    0x00, 0x0C, // Type: PTR
    0x00, 0x01, // Class: IN
];

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_dns_sd_query_all_const_matches_builder() {
        let expected = DNS_SD_QUERY_ALL_BYTES;

        // Compare with new version
        let const_result = build_dns_sd_query_all_().unwrap();
        assert_eq!(const_result, expected);
    }
}
