use hickory_proto::op::{Message, MessageType, OpCode, Query};
use hickory_proto::rr::{Name, RData, RecordType};
use hickory_proto::serialize::binary::BinDecodable as _;
use mds_util::test_expect;
use std::io;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::time::Duration;

pub fn mdns_reverse_lookup(ip: Ipv4Addr) -> io::Result<Option<String>> {
    let msg_bytes = test_expect!(
        build_reverse_dns_query(ip).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    );

    let socket = test_expect!(UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)));
    test_expect!(socket.set_read_timeout(Some(Duration::from_secs(2))));
    test_expect!(socket.send_to(&msg_bytes, mds_util::constants::MDNS_SOCKET_ADDR));

    let mut buf = [0u8; 1500];

    let rcv_data = match socket.recv_from(&mut buf) {
        Ok((len, _src)) => &buf[..len],
        Err(e) => match e.kind() {
            io::ErrorKind::Interrupted => {
                log::warn!("mDNS lookup failed: {e}. Retrying in 1s...");
                std::thread::sleep(Duration::from_secs(1));
                let (len, _src) = socket.recv_from(&mut buf)?;
                &buf[..len]
            }
            _ => return Err(e),
        },
    };

    let response = match Message::from_bytes(rcv_data) {
        Ok(response) => response,
        Err(e) => {
            log::error!(
                "PLEASE SUBMIT BUG REPORT: Protocol error when decoding mDNS lookup response: {e}. Response data={rcv_data:?}"
            );
            return Err(io::Error::new(io::ErrorKind::InvalidData, e));
        }
    };

    for answer in &response.answers {
        if let RData::PTR(name) = &answer.data {
            return Ok(Some(name.to_utf8()));
        }
    }

    Ok(None)
}

fn build_reverse_dns_query(ip: Ipv4Addr) -> Result<Vec<u8>, hickory_proto::ProtoError> {
    let reverse_name = reverse_dns_ptr_record(ip)?;

    let mut message = Message::new(
        mds_util::prelude::MDNS_QUERY_ID,
        MessageType::Query,
        OpCode::Query,
    );
    message.metadata.recursion_desired = false;
    message.add_query(Query::query(reverse_name, RecordType::PTR));
    message.to_vec()
}

#[inline]
fn reverse_dns_ptr_record(ip: Ipv4Addr) -> Result<Name, hickory_proto::ProtoError> {
    const ARPA_SUFFIX: &str = ".in-addr.arpa";
    let [a, b, c, d] = ip.octets();
    let mut reverse_ptr = String::with_capacity("123.123.123.123".len() + ARPA_SUFFIX.len());
    let initial_cap = reverse_ptr.capacity();
    reverse_ptr.push_str(&d.to_string());
    reverse_ptr.push('.');
    reverse_ptr.push_str(&c.to_string());
    reverse_ptr.push('.');
    reverse_ptr.push_str(&b.to_string());
    reverse_ptr.push('.');
    reverse_ptr.push_str(&a.to_string());
    reverse_ptr.push_str(ARPA_SUFFIX);
    let final_cap = reverse_ptr.capacity();
    debug_assert_eq!(initial_cap, final_cap);
    reverse_ptr.parse::<Name>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reverse_record() {
        let ip: Ipv4Addr = "10.200.10.36".parse().unwrap();

        assert!(reverse_dns_ptr_record(ip).is_ok());
        assert!(build_reverse_dns_query(ip).is_ok());
    }
}
