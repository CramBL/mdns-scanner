use hickory_proto::op::{Message, MessageType, OpCode, Query};
use hickory_proto::rr::{Name, RData, RecordType};
use hickory_proto::serialize::binary::BinDecodable as _;
use mds_log::prelude::Logger;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::time::Duration;

pub fn mdns_reverse_lookup(log: &Logger, ip: Ipv4Addr) -> anyhow::Result<Option<String>> {
    let msg_bytes = build_reverse_dns_query(ip)?;

    let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))?;
    socket.set_read_timeout(Some(Duration::from_millis(800)))?;
    socket.send_to(&msg_bytes, mds_util::constants::MDNS_SOCKET_ADDR)?;

    let mut buf = [0u8; 1500];

    let (len, _src) = socket.recv_from(&mut buf)?;

    let response = Message::from_bytes(&buf[..len])?;

    log.info(format!("mDNS reverse lookup: {response:?}"));

    for answer in response.answers() {
        if let RData::PTR(name) = answer.data() {
            return Ok(Some(name.to_utf8()));
        }
    }

    Ok(None)
}

fn build_reverse_dns_query(ip: Ipv4Addr) -> anyhow::Result<Vec<u8>> {
    let reverse_name = reverse_dns_ptr_record(ip)?;

    let mut message = Message::new();
    message
        .set_id(mds_util::prelude::MDNS_QUERY_ID)
        .set_message_type(MessageType::Query)
        .set_op_code(OpCode::Query)
        .set_recursion_desired(false)
        .add_query(Query::query(reverse_name, RecordType::PTR));
    Ok(message.to_vec()?)
}

#[inline]
fn reverse_dns_ptr_record(ip: Ipv4Addr) -> anyhow::Result<Name> {
    const ARPA_SUFFIX: &str = ".in-addr.arpa";
    let [a, b, c, d] = ip.octets();
    let mut reverse_ptr = String::with_capacity("123.123.123.123".len() + ARPA_SUFFIX.len());
    reverse_ptr.push_str(&d.to_string());
    reverse_ptr.push('.');
    reverse_ptr.push_str(&c.to_string());
    reverse_ptr.push('.');
    reverse_ptr.push_str(&b.to_string());
    reverse_ptr.push('.');
    reverse_ptr.push_str(&a.to_string());
    reverse_ptr.push_str(ARPA_SUFFIX);
    Ok(reverse_ptr.parse::<Name>()?)
}
