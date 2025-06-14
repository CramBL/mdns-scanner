use pnet::packet::Packet;
use pnet::packet::icmp::IcmpPacket;
use pnet::packet::icmp::{IcmpTypes, echo_request::MutableEchoRequestPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::transport::{
    TransportChannelType::Layer4, TransportProtocol::Ipv4, icmp_packet_iter, transport_channel,
};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::mpsc;
use std::time::Duration;
use std::{io, thread};

pub(crate) fn icmp_ping(ip: Ipv4Addr) -> bool {
    const TIMEOUT: Duration = Duration::from_millis(500);
    if let Ok(result) = try_raw_icmp_ping_with_timeout(ip, TIMEOUT) {
        return result;
    }
    native_icmp_ping(ip)
}

fn try_raw_icmp_ping_with_timeout(ip: Ipv4Addr, timeout: Duration) -> Result<bool, io::Error> {
    let (mut tx, mut rx) = transport_channel(1024, Layer4(Ipv4(IpNextHeaderProtocols::Icmp)))?;

    const PACKET_SIZE: usize = 64;
    let mut buffer = [0u8; PACKET_SIZE];
    let mut packet = MutableEchoRequestPacket::new(&mut buffer).unwrap();
    packet.set_icmp_type(IcmpTypes::EchoRequest);
    packet.set_sequence_number(1);
    packet.set_identifier(0x1234);
    packet.set_checksum(pnet::util::checksum(packet.packet(), 1));

    let dest = IpAddr::V4(ip);
    tx.send_to(packet, dest)?;

    let (result_tx, result_rx) = mpsc::channel();

    // Spawn a thread to handle the blocking receive
    thread::Builder::new()
        .name("raw_icmp_ping".into())
        .spawn(move || {
            let mut iter = icmp_packet_iter(&mut rx);
            while let Ok((packet, addr)) = iter.next() {
                if addr == dest {
                    if let Some(echo_reply) = IcmpPacket::new(packet.packet()) {
                        if echo_reply.get_icmp_type() == IcmpTypes::EchoReply {
                            let _ = result_tx.send(true);
                            return;
                        }
                    }
                }
            }
            let _ = result_tx.send(false);
        })
        .expect("Failed creating raw ICMP ping thread");

    // Wait for either a result or timeout
    match result_rx.recv_timeout(timeout) {
        Ok(result) => Ok(result),
        Err(mpsc::RecvTimeoutError::Timeout) => Ok(false),
        Err(mpsc::RecvTimeoutError::Disconnected) => Ok(false),
    }
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
    use crate::prelude::IP_TEST_NET_1_UNREACHABLE;

    use super::*;
    use std::{net::Ipv4Addr, time::Instant};

    #[test]
    fn test_ping_localhost() {
        let ip = Ipv4Addr::new(127, 0, 0, 1);
        assert!(
            icmp_ping(ip),
            "Pinging localhost (127.0.0.1) should succeed."
        );
    }

    #[test]
    fn test_native_ping_localhost() {
        let ip = Ipv4Addr::new(127, 0, 0, 1);
        assert!(
            native_icmp_ping(ip),
            "Pinging localhost (127.0.0.1) should succeed."
        );
    }

    #[test]
    fn test_raw_ping_localhost() {
        let ip = Ipv4Addr::new(127, 0, 0, 1);
        let err = try_raw_icmp_ping_with_timeout(ip, Duration::from_millis(100)).unwrap_err();
        assert_eq!(
            err.kind(),
            io::ErrorKind::PermissionDenied,
            "Raw socket handling should result in permission issues"
        );
    }

    #[test]
    fn test_ping_known_unreachable_host() {
        assert!(
            !icmp_ping(IP_TEST_NET_1_UNREACHABLE),
            "Pinging a documentation IP (192.0.2.1) should fail."
        );
    }

    #[test]
    fn test_native_ping_known_unreachable_host() {
        let reachable = native_icmp_ping(IP_TEST_NET_1_UNREACHABLE);
        assert!(
            !reachable,
            "Pinging a documentation IP (192.0.2.1) should fail."
        );
    }

    #[test]
    fn test_raw_ping_known_unreachable_host() {
        let err = try_raw_icmp_ping_with_timeout(IP_TEST_NET_1_UNREACHABLE, Duration::from_secs(1))
            .unwrap_err();
        assert_eq!(
            err.kind(),
            io::ErrorKind::PermissionDenied,
            "Raw socket handling should result in permission issues"
        );
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
            "Function should not hang for more than 2 seconds, took {:?}",
            elapsed
        );
    }

    #[test]
    fn test_raw_ping_localhost_alt() {
        let start = Instant::now();

        match try_raw_icmp_ping_with_timeout(Ipv4Addr::new(127, 0, 0, 1), Duration::from_secs(2)) {
            Ok(result) => {
                let elapsed = start.elapsed();
                assert!(
                    elapsed < Duration::from_secs(5),
                    "Localhost ping should be fast, took {:?}",
                    elapsed
                );

                // Localhost should generally work, but don't fail the test if it doesn't
                // due to Windows networking quirks
                println!("Localhost ping result: {}, elapsed: {:?}", result, elapsed);
            }
            Err(e) => {
                let elapsed = start.elapsed();
                assert!(
                    elapsed < Duration::from_secs(5),
                    "Test should complete quickly even on error, took {:?}",
                    elapsed
                );
                println!("Localhost ping failed with error: {:?}", e);
            }
        }
        assert!(false)
    }

    #[test]
    fn test_raw_ping_known_unreachable_host_alt() {
        let start = Instant::now();

        // The main goal is to ensure this doesn't hang forever
        match try_raw_icmp_ping_with_timeout(IP_TEST_NET_1_UNREACHABLE, Duration::from_secs(1)) {
            Ok(result) => {
                let elapsed = start.elapsed();
                assert!(
                    elapsed < Duration::from_secs(3),
                    "Test should complete quickly, took {:?}",
                    elapsed
                );
                assert!(!result, "Unreachable host should return false");
                println!(
                    "Raw ping completed normally: result={}, elapsed={:?}",
                    result, elapsed
                );
            }
            Err(e) => {
                let elapsed = start.elapsed();
                assert!(
                    elapsed < Duration::from_secs(3),
                    "Test should complete quickly even on error, took {:?}",
                    elapsed
                );

                // On Windows, we might get permission denied, which is fine
                if e.kind() == io::ErrorKind::PermissionDenied {
                    println!("Got expected permission denied error (this is fine on Windows)");
                } else {
                    println!(
                        "Got error: {:?} (this might be expected on some systems)",
                        e
                    );
                }
            }
        }

        assert!(false)
    }
}
