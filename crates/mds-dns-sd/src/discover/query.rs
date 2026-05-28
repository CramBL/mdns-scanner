use crate::discover::UdpSocketSender;
use crate::service_registry::ServiceRegistry;
use hickory_proto::op::{Message, Query};
use hickory_proto::rr::{Name, RecordType};
use mds_util::prelude::*;
use std::io;
use std::net::UdpSocket;
use std::str::FromStr;

pub(crate) struct DnsRequester<'a, S: UdpSocketSender> {
    socket: &'a S,
}

impl<'a, S: UdpSocketSender> DnsRequester<'a, S> {
    pub(super) fn new(socket: &'a S) -> Self {
        Self { socket }
    }

    pub(super) fn query_ptr(&self, service_type: &Name) -> io::Result<()> {
        let query = build_query(service_type, &[RecordType::PTR])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.send(&query)
    }

    pub(super) fn query_srv_and_txt(&self, instance_name: &Name) -> io::Result<()> {
        let query = build_query(instance_name, &[RecordType::SRV, RecordType::TXT])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.send(&query)
    }

    pub(super) fn query_a_and_aaaa(&self, hostname: &Name) -> io::Result<()> {
        let query = build_query(hostname, &[RecordType::A, RecordType::AAAA])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.send(&query)
    }

    fn send(&self, query: &[u8]) -> io::Result<()> {
        self.socket.send_to(query, MDNS_SOCKET_ADDR)?;
        Ok(())
    }
}

pub(super) fn send_mdns_query(
    query: &[u8],
    socket: &UdpSocket,
    registry: &mut ServiceRegistry,
) -> io::Result<()> {
    socket.send_to(query, MDNS_SOCKET_ADDR)?;
    let mut buf = [0u8; 1500];

    while let Ok((len, _src)) = socket.recv_from(&mut buf) {
        let received_data = &buf[..len];
        match super::parse_dns_response(received_data) {
            Ok(msg) => test_expect!(super::handle_mdns_response(&msg, socket, registry)),
            Err(e) => log::warn!("mDNS response handling error: {e}"),
        }
    }
    Ok(())
}

#[allow(dead_code, reason = "used to confirm that the const buffer is correct")]
pub(super) fn build_dns_sd_query_all_() -> Result<Vec<u8>, hickory_proto::ProtoError> {
    build_query(
        &Name::from_str(DNS_SD_QUERY_ALL).unwrap(),
        &[RecordType::PTR],
    )
}

fn build_query(
    name: &Name,
    record_types: &[RecordType],
) -> Result<Vec<u8>, hickory_proto::ProtoError> {
    use hickory_proto::op::{MessageType, OpCode};
    let mut message = Message::new(MDNS_QUERY_ID, MessageType::Query, OpCode::Query);
    message.metadata.recursion_desired = false;

    for &record_type in record_types {
        let q = Query::query(name.clone(), record_type);
        message.add_query(q);
    }

    message.to_vec()
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
        let const_result = build_dns_sd_query_all_().unwrap();
        assert_eq!(const_result, expected);
    }
}
