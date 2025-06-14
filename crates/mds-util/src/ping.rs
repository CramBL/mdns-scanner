use pnet::packet::Packet;
use pnet::packet::icmp::IcmpPacket;
use pnet::packet::icmp::{IcmpTypes, echo_request::MutableEchoRequestPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::transport::{
    TransportChannelType::Layer4, TransportProtocol::Ipv4, icmp_packet_iter, transport_channel,
};
use std::net::{IpAddr, Ipv4Addr};
use std::time::{Duration, Instant};

pub(crate) fn icmp_ping(ip: Ipv4Addr) -> bool {
    const TIMEOUT: Duration = Duration::from_millis(500);
    const PACKET_SIZE: usize = 64;

    // Create a transport channel for ICMP
    let (mut tx, mut rx) = match transport_channel(1024, Layer4(Ipv4(IpNextHeaderProtocols::Icmp)))
    {
        Ok((tx, rx)) => (tx, rx),
        Err(e) => {
            match e.kind() {
                std::io::ErrorKind::PermissionDenied => {
                    // We couldn't do it because we need root, so let's just try to run the system 'ping' binary
                    return native_icmp_ping(ip);
                }
                _ => return false,
            }
        }
    };

    let mut buffer = [0u8; PACKET_SIZE];
    let mut packet = MutableEchoRequestPacket::new(&mut buffer).unwrap();
    packet.set_icmp_type(IcmpTypes::EchoRequest);
    packet.set_sequence_number(1);
    packet.set_identifier(0x1234);
    packet.set_checksum(pnet::util::checksum(packet.packet(), 1));

    let dest = IpAddr::V4(ip);
    if tx.send_to(packet, dest).is_err() {
        return false;
    }

    let start = Instant::now();
    let mut iter = icmp_packet_iter(&mut rx);
    while start.elapsed() < TIMEOUT {
        if let Ok((packet, addr)) = iter.next() {
            if addr == dest {
                if let Some(echo_reply) = IcmpPacket::new(packet.packet()) {
                    if echo_reply.get_icmp_type() == IcmpTypes::EchoReply {
                        return true;
                    }
                }
            }
        }
    }

    false
}

fn native_icmp_ping(ip: Ipv4Addr) -> bool {
    let ip_str = ip.to_string();

    #[cfg(unix)]
    let output = {
        const TIMEOUT: &str = "0.5"; // s
        std::process::Command::new("ping")
            .arg("-c")
            .arg("1")
            .arg("-W")
            .arg(TIMEOUT)
            .arg(ip_str)
            .output()
    };

    #[cfg(windows)]
    let output = {
        const TIMEOUT: &str = "500"; // ms
        std::process::Command::new("ping")
            .arg("-n")
            .arg("1")
            .arg("-w")
            .arg(TIMEOUT)
            .arg(ip_str)
            .output()
    };

    if let Ok(output) = output {
        #[cfg(unix)]
        return output.status.success();

        // On Windows, we need to parse the output text...
        #[cfg(windows)]
        {
            // Check for "Reply from" which indicates a successful ping
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.contains("Reply from")
        }
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_ping_localhost() {
        let ip = Ipv4Addr::new(127, 0, 0, 1);
        assert!(
            icmp_ping(ip),
            "Pinging localhost (127.0.0.1) should succeed."
        );
    }

    /// 192.0.2.1 is from the TEST-NET-1 range reserved for documentation (RFC 5737).
    /// It should never be reachable, making it suitable for testing failure cases.
    /// This test validates that the function correctly times out and returns false.
    #[test]
    fn test_ping_known_unreachable_host() {
        let ip = Ipv4Addr::new(192, 0, 2, 1);
        assert!(
            !icmp_ping(ip),
            "Pinging a documentation IP (192.0.2.1) should fail."
        );
    }

    /// Ping a highly available public DNS server (Cloudflare).
    #[test]
    fn test_ping_reliable_public_host() {
        let ip = Ipv4Addr::new(1, 1, 1, 1);
        assert!(
            icmp_ping(ip),
            "Pinging a reliable public host (1.1.1.1) should succeed."
        );
    }
}
