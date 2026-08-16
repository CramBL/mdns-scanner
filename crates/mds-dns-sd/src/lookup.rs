use hickory_proto::op::{Message, MessageType, OpCode, Query};
use hickory_proto::rr::{Name, RData, RecordType};
use hickory_proto::serialize::binary::BinDecodable as _;
use mds_util::constants::{DNS_SD_QUERY_ALL, MULTICAST_ADDR, MULTICAST_PORT};
use mds_util::test_expect;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const READ_TIMEOUT: Duration = Duration::from_secs(2);

const PROBE_POLL_INTERVAL: Duration = Duration::from_millis(5);

pub fn mdns_reverse_lookup(ip: Ipv4Addr) -> io::Result<Option<String>> {
    let msg_bytes = test_expect!(
        build_reverse_dns_query(ip).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    );

    let socket = test_expect!(bind_ephemeral_mdns_socket());
    test_expect!(socket.set_read_timeout(Some(READ_TIMEOUT)));
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

/// How a host replied to an mDNS probe within the timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MdnsReplyMode {
    Silent,
    Unicast,
    Multicast,
    UnicastAndMulticast,
}

impl MdnsReplyMode {
    fn from_replies(via_unicast: bool, via_multicast: bool) -> Self {
        match (via_unicast, via_multicast) {
            (false, false) => Self::Silent,
            (true, false) => Self::Unicast,
            (false, true) => Self::Multicast,
            (true, true) => Self::UnicastAndMulticast,
        }
    }
}

/// Sends a unicast DNS-SD meta-query to `<ip>:5353` and reports how the host answers
/// within `timeout`: not at all, or via one or both mDNS reply modes.
///
/// A responder may reply either way, so the probe listens on two sockets, recording each
/// matching [MessageType::Response] whose source IP is `ip`:
/// - a legacy-unicast reply on our ephemeral source port ([ProbeReplyMode::LegacyUnicast]),
/// - a multicast reply on the joined 224.0.0.251:5353 group ([ProbeReplyMode::Multicast]).
///
/// It waits out the window to observe both modes, returning early once both are seen or
/// once `cancel` is set, so a cancelled scan stops probing promptly.
pub fn probe_mdns_responder(
    ip: Ipv4Addr,
    timeout: Duration,
    cancel: &AtomicBool,
) -> io::Result<MdnsReplyMode> {
    let query =
        build_dns_sd_meta_query().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let legacy_unicast_socket = bind_ephemeral_mdns_socket()?;
    legacy_unicast_socket.set_nonblocking(true)?;
    let multicast_socket = bind_multicast_mdns_socket()?;

    // Per RFC 6762 §6.7 the responder must unicast its reply directly back to this
    // ephemeral socket, since our source port is not 5353.
    let destination = SocketAddrV4::new(ip, MULTICAST_PORT);
    legacy_unicast_socket.send_to(&query, destination)?;

    let deadline = Instant::now() + timeout;
    let mut buf = [0u8; 1500];
    let mut via_unicast = false;
    let mut via_multicast = false;
    while Instant::now() < deadline
        && !(via_unicast && via_multicast)
        && !cancel.load(Ordering::Relaxed)
    {
        if !via_unicast
            && poll_probe_socket(
                &legacy_unicast_socket,
                ProbeReplyMode::LegacyUnicast,
                ip,
                &mut buf,
            )?
        {
            via_unicast = true;
        }
        if !via_multicast
            && poll_probe_socket(&multicast_socket, ProbeReplyMode::Multicast, ip, &mut buf)?
        {
            via_multicast = true;
        }

        std::thread::sleep(PROBE_POLL_INTERVAL);
    }

    Ok(MdnsReplyMode::from_replies(via_unicast, via_multicast))
}

fn poll_probe_socket(
    socket: &UdpSocket,
    mode: ProbeReplyMode,
    expected_source_ip: Ipv4Addr,
    buf: &mut [u8],
) -> io::Result<bool> {
    match socket.recv_from(buf) {
        Ok((len, src)) => Ok(mode.received_matching_response(expected_source_ip, src, &buf[..len])),
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(false),
        Err(e) => Err(e),
    }
}

/// The reply mode a probe socket listens for, which governs how it treats datagrams
/// from sources other than the probe target.
#[derive(Clone, Copy)]
enum ProbeReplyMode {
    /// RFC 6762 §6.7 "Legacy Unicast Responses" (<https://www.rfc-editor.org/rfc/rfc6762#section-6.7>):
    /// the responder unicasts its reply back to our ephemeral port because the query's source
    /// port is not 5353.
    ///
    /// "If the source UDP port in a received Multicast DNS query is not port 5353, this
    /// indicates that the querier originating the query is a simple resolver ... which does
    /// not fully implement all of Multicast DNS." and "the Multicast DNS responder MUST send
    /// a UDP response directly back to the querier, via unicast, to the query packet's source
    /// IP address and port."
    ///
    /// Only the target sends to our ephemeral port: an undecodable datagram is surfaced with
    /// `log::warn!`, every other unexpected datagram with `log::debug!`.
    LegacyUnicast,

    /// RFC 6762 §6 (<https://www.rfc-editor.org/rfc/rfc6762#section-6>): we join 224.0.0.251:5353
    /// to catch a reply sent there, since multicast is the default response mode.
    ///
    /// "Except for these three specific cases, responses MUST NOT be sent via unicast ..."
    ///
    /// The query is a direct unicast query to port 5353 per RFC 6762 §5.5 "Direct Unicast Queries
    /// to Port 5353" (<https://www.rfc-editor.org/rfc/rfc6762#section-5.5>): "When a Multicast DNS
    /// responder receives a query via direct unicast, it SHOULD respond as it would for 'QU'
    /// questions."
    ///
    /// This socket receives every device's mDNS multicast traffic: datagrams from a source other
    /// than the target are background noise and are dropped without logging.
    Multicast,
}

impl ProbeReplyMode {
    fn received_matching_response(self, ip: Ipv4Addr, src: SocketAddr, datagram: &[u8]) -> bool {
        let from_target = src.ip() == IpAddr::V4(ip);
        if matches!(self, Self::Multicast) && !from_target {
            // Background noise from another device on the shared multicast group.
            return false;
        }

        match classify_probe_datagram(datagram) {
            ProbeDatagram::Response if from_target => true,
            ProbeDatagram::Response => {
                log::debug!(
                    "mDNS probe of {ip}: dropping a response from a different source {src}"
                );
                false
            }
            ProbeDatagram::NonResponse(message_type) => {
                log::debug!("mDNS probe of {ip}: dropping a {message_type:?} datagram from {src}");
                false
            }
            ProbeDatagram::Undecodable(e) => {
                log::warn!(
                    "mDNS probe of {ip}: {src} sent an undecodable datagram ({} bytes): {e}",
                    datagram.len()
                );
                false
            }
        }
    }
}

/// What a datagram received during a probe amounts to.
enum ProbeDatagram {
    Response,
    NonResponse(MessageType),
    Undecodable(String),
}

fn classify_probe_datagram(datagram: &[u8]) -> ProbeDatagram {
    match Message::from_bytes(datagram) {
        Ok(message) if message.message_type == MessageType::Response => ProbeDatagram::Response,
        Ok(message) => ProbeDatagram::NonResponse(message.message_type),
        Err(e) => ProbeDatagram::Undecodable(e.to_string()),
    }
}

fn bind_ephemeral_mdns_socket() -> io::Result<UdpSocket> {
    UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
}

fn bind_multicast_mdns_socket() -> io::Result<UdpSocket> {
    let socket = crate::setup_socket()?;
    // Joins the group to receive a reply sent there: RFC 6762 §6 makes multicast the
    // default mDNS response mode.
    socket.join_multicast_v4(&MULTICAST_ADDR, &Ipv4Addr::UNSPECIFIED)?;
    socket.set_nonblocking(true)?;
    Ok(socket)
}

fn build_reverse_dns_query(ip: Ipv4Addr) -> Result<Vec<u8>, hickory_proto::ProtoError> {
    build_mdns_query(reverse_dns_ptr_record(ip)?, RecordType::PTR)
}

fn build_dns_sd_meta_query() -> Result<Vec<u8>, hickory_proto::ProtoError> {
    build_mdns_query(DNS_SD_QUERY_ALL.parse::<Name>()?, RecordType::PTR)
}

fn build_mdns_query(
    name: Name,
    record_type: RecordType,
) -> Result<Vec<u8>, hickory_proto::ProtoError> {
    let mut message = Message::new(
        mds_util::prelude::MDNS_QUERY_ID,
        MessageType::Query,
        OpCode::Query,
    );
    message.metadata.recursion_desired = false;
    message.add_query(Query::query(name, record_type));
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
    use rstest::rstest;

    const TARGET: Ipv4Addr = Ipv4Addr::new(192, 168, 0, 42);
    const OTHER_SOURCE: Ipv4Addr = Ipv4Addr::new(192, 168, 0, 99);

    fn source(ip: Ipv4Addr) -> SocketAddr {
        SocketAddr::V4(SocketAddrV4::new(ip, MULTICAST_PORT))
    }

    fn response_bytes() -> Vec<u8> {
        Message::new(
            mds_util::prelude::MDNS_QUERY_ID,
            MessageType::Response,
            OpCode::Query,
        )
        .to_vec()
        .expect("response serializes")
    }

    #[test]
    fn test_reverse_record() {
        let ip: Ipv4Addr = "10.200.10.36".parse().unwrap();

        assert!(reverse_dns_ptr_record(ip).is_ok());
        assert!(build_reverse_dns_query(ip).is_ok());
    }

    #[test]
    fn dns_sd_meta_query_builds() {
        assert!(build_dns_sd_meta_query().is_ok());
    }

    #[test]
    fn probe_datagrams_are_classified() {
        let query = build_dns_sd_meta_query().expect("meta query builds");
        assert!(
            matches!(
                classify_probe_datagram(&query),
                ProbeDatagram::NonResponse(_)
            ),
            "an outgoing query is not a response"
        );

        assert!(matches!(
            classify_probe_datagram(&response_bytes()),
            ProbeDatagram::Response
        ));

        assert!(
            matches!(
                classify_probe_datagram(&[0xff, 0x13, 0x00]),
                ProbeDatagram::Undecodable(_)
            ),
            "undecodable bytes are reported as undecodable"
        );
    }

    #[rstest]
    #[case(ProbeReplyMode::LegacyUnicast, TARGET, true)]
    #[case(ProbeReplyMode::LegacyUnicast, OTHER_SOURCE, false)]
    #[case(ProbeReplyMode::Multicast, TARGET, true)]
    #[case(ProbeReplyMode::Multicast, OTHER_SOURCE, false)]
    fn only_a_response_from_the_target_matches(
        #[case] mode: ProbeReplyMode,
        #[case] response_source: Ipv4Addr,
        #[case] expected_match: bool,
    ) {
        let matched =
            mode.received_matching_response(TARGET, source(response_source), &response_bytes());
        assert_eq!(matched, expected_match);
    }

    #[test]
    fn probe_of_unreachable_host_reports_no_response() {
        let reply_mode = probe_mdns_responder(
            mds_util::prelude::IP_TEST_NET_1_UNREACHABLE,
            Duration::from_millis(200),
            &AtomicBool::new(false),
        )
        .expect("probe of an unreachable host should time out, not error");
        assert_eq!(reply_mode, MdnsReplyMode::Silent);
    }

    #[test]
    fn cancelling_a_running_probe_stops_it_promptly() {
        // A 10s window the probe would otherwise sit out against an unreachable host.
        // Cancel it after it has entered its polling loop and confirm it returns quickly.
        let cancel = AtomicBool::new(false);
        let probe_duration = std::thread::scope(|scope| {
            let probe = scope.spawn(|| {
                let start = Instant::now();
                probe_mdns_responder(
                    mds_util::prelude::IP_TEST_NET_1_UNREACHABLE,
                    Duration::from_secs(10),
                    &cancel,
                )
                .expect("a cancelled probe should return, not error");
                start.elapsed()
            });
            std::thread::sleep(Duration::from_millis(100));
            cancel.store(true, Ordering::Relaxed);
            probe.join().expect("probe thread panicked")
        });
        assert!(
            probe_duration < Duration::from_secs(2),
            "a probe cancelled mid-run must stop well before its 10s window, took {probe_duration:?}"
        );
    }

    #[rstest]
    #[case(false, false, MdnsReplyMode::Silent)]
    #[case(true, false, MdnsReplyMode::Unicast)]
    #[case(false, true, MdnsReplyMode::Multicast)]
    #[case(true, true, MdnsReplyMode::UnicastAndMulticast)]
    fn reply_mode_reflects_which_sockets_answered(
        #[case] via_unicast: bool,
        #[case] via_multicast: bool,
        #[case] expected: MdnsReplyMode,
    ) {
        assert_eq!(
            MdnsReplyMode::from_replies(via_unicast, via_multicast),
            expected
        );
    }
}
