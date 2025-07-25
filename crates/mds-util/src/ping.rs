use pnet::packet::Packet;
use pnet::packet::icmp::{IcmpTypes, echo_request::MutableEchoRequestPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::transport::TransportReceiver;
use pnet::transport::{
    TransportChannelType::Layer4, TransportProtocol::Ipv4, icmp_packet_iter, transport_channel,
};
use std::net::{IpAddr, Ipv4Addr};

use std::io;
use std::time::{Duration, Instant};

use crate::test_expect;

pub fn icmp_ping(ip: Ipv4Addr, timeout: Duration) -> Option<Duration> {
    match try_raw_icmp_ping_with_timeout(ip, timeout) {
        Ok(reachable) => reachable,
        Err(e) => {
            match e.kind() {
                io::ErrorKind::TimedOut => None,
                // Generally we only expect to get permission denied if we're not allowed to
                // create raw sockets, but in any case, we fall back on the native ping binary
                _ => {
                    let now = Instant::now();
                    if native_icmp_ping(ip, timeout) {
                        Some(now.elapsed())
                    } else {
                        None
                    }
                }
            }
        }
    }
}

fn try_raw_icmp_ping_with_timeout(
    ip: Ipv4Addr,
    timeout: Duration,
) -> Result<Option<Duration>, io::Error> {
    const CHANNEL_BUFFER_SIZE: usize = 128;
    let (mut transport_tx, transport_rx) = transport_channel(
        CHANNEL_BUFFER_SIZE,
        Layer4(Ipv4(IpNextHeaderProtocols::Icmp)),
    )?;

    const PACKET_SIZE: usize = 64;
    let mut buffer = [0u8; PACKET_SIZE];
    let mut packet = MutableEchoRequestPacket::new(&mut buffer).unwrap();
    packet.set_icmp_type(IcmpTypes::EchoRequest);
    packet.set_sequence_number(1);
    packet.set_identifier(0x1234);
    packet.set_checksum(pnet::util::checksum(packet.packet(), 1));

    let dest = IpAddr::V4(ip);
    test_expect!(transport_tx.send_to(packet, dest));

    let now = std::time::Instant::now();
    let res = {
        #[cfg(windows)]
        {
            win_do_raw_icmp_ping(dest, timeout, transport_rx)
        }
        #[cfg(not(windows))]
        {
            unix_do_raw_icmp_ping(dest, timeout, transport_rx)
        }
    };
    let elapsed = now.elapsed();
    let reachable = if res? { Some(elapsed) } else { None };
    Ok(reachable)
}

#[cfg(not(windows))]
fn unix_do_raw_icmp_ping(
    dest: IpAddr,
    timeout: Duration,
    mut transport_rx: TransportReceiver,
) -> Result<bool, io::Error> {
    let now = std::time::Instant::now();
    let mut iter = icmp_packet_iter(&mut transport_rx);
    while let Ok(recv) = iter.next_with_timeout(timeout) {
        if recv.is_some_and(|(packet, addr)| {
            packet.get_icmp_type() == IcmpTypes::EchoReply && addr == dest
        }) {
            return Ok(true);
        }
        if now.elapsed() >= timeout {
            // If for some reason there's a host who continuously sends us ICMP packets
            return Ok(false);
        }
    }
    Ok(false)
}

#[cfg(windows)]
fn win_do_raw_icmp_ping(
    dest: IpAddr,
    timeout: Duration,
    mut transport_rx: TransportReceiver,
) -> Result<bool, io::Error> {
    use std::sync::mpsc;
    let (result_tx, result_rx) = mpsc::channel();

    // Spawn a thread to handle the blocking receive
    std::thread::Builder::new()
        .name("raw_icmp_ping".into())
        .spawn(move || {
            let mut iter = icmp_packet_iter(&mut transport_rx);
            while let Ok((packet, addr)) = iter.next() {
                if packet.get_icmp_type() == IcmpTypes::EchoReply && addr == dest {
                    let _ = result_tx.send(true);
                    return;
                }
            }
            let _ = result_tx.send(false);
        })
        .expect("Failed creating raw ICMP ping thread");

    // Wait for either a result or timeout
    match result_rx.recv_timeout(timeout) {
        Ok(result) => Ok(result),
        Err(mpsc::RecvTimeoutError::Timeout) | Err(mpsc::RecvTimeoutError::Disconnected) => {
            Ok(false)
        }
    }
}

fn native_icmp_ping(ip: Ipv4Addr, timeout: Duration) -> bool {
    let ip_str = ip.to_string();

    #[cfg(unix)]
    let output = {
        let timeout_str: String = timeout.as_secs_f32().to_string();
        std::process::Command::new("ping")
            .arg("-c")
            .arg("1")
            .arg("-W")
            .arg(timeout_str)
            .arg(ip_str)
            .output()
    };

    #[cfg(windows)]
    let output = {
        let timeout_str = timeout.as_millis().to_string();
        std::process::Command::new("ping")
            .arg("-n")
            .arg("1")
            .arg("-w")
            .arg(timeout_str)
            .arg(ip_str)
            .output()
    };

    if let Ok(output) = output {
        #[cfg(unix)]
        return output.status.success();

        // On Windows, we need to parse the output text...
        #[cfg(windows)]
        is_reachable_from_ping_output(&output.stdout)
    } else {
        false
    }
}

#[cfg(windows)]
fn is_reachable_from_ping_output(stdout: &[u8]) -> bool {
    // Must have "Reply from" first
    if !stdout.windows(10).any(|w| w == b"Reply from") {
        return false;
    }

    // Check for negative reply types (these are BAD even with "Reply from")
    let negative_replies: &[&[u8]] = &[
        b"unreachable",
        b"TTL expired in transit",
        b"TTL expired during reassembly",
        b"Parameter problem",
        b"Source quench",
        b"Packet filtered",
        b"Host prohibited",
        b"Communication prohibited",
        b"Precedence cutoff in effect",
    ];

    // If we find any negative reply type, it's unreachable
    for &pattern in negative_replies {
        if stdout.windows(pattern.len()).any(|w| w == pattern) {
            return false;
        }
    }

    // Now check for positive indicators in the reply
    let has_bytes = stdout.windows(6).any(|w| w == b"bytes=");
    let has_time =
        stdout.windows(5).any(|w| w == b"time<") || stdout.windows(5).any(|w| w == b"time=");
    let has_ttl = stdout.windows(4).any(|w| w == b"TTL=");

    // A successful ping reply MUST have bytes= and either time= or TTL=
    has_bytes && (has_time || has_ttl)
}

#[cfg(test)]
mod tests {
    use crate::prelude::IP_TEST_NET_1_UNREACHABLE;

    use super::*;
    use std::{net::Ipv4Addr, time::Instant};

    #[test]
    fn test_ping_localhost() {
        let ip = Ipv4Addr::new(127, 0, 0, 1);
        let reached_in = icmp_ping(ip, Duration::from_millis(100));
        assert!(
            reached_in.is_some(),
            "Pinging localhost (127.0.0.1) should succeed."
        );
    }

    #[test]
    fn test_native_ping_localhost() {
        let ip = Ipv4Addr::new(127, 0, 0, 1);
        assert!(
            native_icmp_ping(ip, Duration::from_millis(300)),
            "Pinging localhost (127.0.0.1) should succeed."
        );
    }

    #[test]
    fn test_raw_ping_localhost() {
        let ip = Ipv4Addr::new(127, 0, 0, 1);

        let res = try_raw_icmp_ping_with_timeout(ip, Duration::from_millis(100));
        if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
            let err = res.unwrap_err();
            assert_eq!(
                err.kind(),
                io::ErrorKind::PermissionDenied,
                "Raw socket handling should result in permission issues on linux and macos"
            );
        } else if cfg!(target_os = "windows") {
            let reachable = res.unwrap();
            assert!(
                reachable.is_some(),
                "raw ping to localhost should return reachable on windows"
            );
        }
    }

    #[test]
    fn test_ping_known_unreachable_host() {
        assert!(
            icmp_ping(IP_TEST_NET_1_UNREACHABLE, Duration::from_millis(500)).is_none(),
            "Pinging a documentation IP (192.0.2.1) should fail."
        );
    }

    #[test]
    fn test_native_ping_known_unreachable_host() {
        let reachable = native_icmp_ping(IP_TEST_NET_1_UNREACHABLE, Duration::from_millis(400));
        assert!(
            !reachable,
            "Pinging a documentation IP (192.0.2.1) should fail."
        );
    }

    #[test]
    fn test_raw_ping_known_unreachable_host() {
        let res = try_raw_icmp_ping_with_timeout(IP_TEST_NET_1_UNREACHABLE, Duration::from_secs(1));
        if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
            let err = res.unwrap_err();
            assert_eq!(
                err.kind(),
                io::ErrorKind::PermissionDenied,
                "Raw socket handling should result in permission issues on macos and linux"
            );
        } else if cfg!(target_os = "windows") {
            let reached_in = res.unwrap();
            assert!(
                reached_in.is_none(),
                "raw ping to {IP_TEST_NET_1_UNREACHABLE} (unreachable test IP) should return unreachable on windows"
            );
        }
    }

    /// This test just ensures the function returns within a reasonable time
    #[test]
    fn test_no_hanging_on_unreachable_host() {
        let start = Instant::now();

        let _ =
            try_raw_icmp_ping_with_timeout(IP_TEST_NET_1_UNREACHABLE, Duration::from_millis(500));

        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_secs(1),
            "Function should not hang for more than 2 seconds, took {elapsed:?}"
        );
    }
}
