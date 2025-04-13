use pnet::packet::Packet;
use pnet::packet::icmp::IcmpPacket;
use pnet::packet::icmp::{IcmpTypes, echo_request::MutableEchoRequestPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::transport::{
    TransportChannelType::Layer4, TransportProtocol::Ipv4, icmp_packet_iter, transport_channel,
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use crate::log::Logger;

const SSH_PORT: u16 = 22;
const HTTP_PORT: u16 = 80;
const HTTPS_PORT: u16 = 443;
const SCAN_PORTS: &[u16] = &[SSH_PORT, HTTP_PORT, HTTPS_PORT];

pub(crate) fn is_host_up(mut log: Logger, ip: Ipv4Addr) -> bool {
    log.trace(format!("Checking if a host is up at {ip}"));
    let icmp_handle = thread::spawn({
        let ip = ip.clone();
        move || icmp_ping(ip)
    });

    let tcp_handle = thread::spawn({
        let ip = ip.clone();
        move || {
            for port in SCAN_PORTS {
                let socket_addr = SocketAddr::new(IpAddr::V4(ip), *port);
                if TcpStream::connect_timeout(&socket_addr, Duration::from_millis(100)).is_ok() {
                    return true;
                }
            }
            false
        }
    });

    let mut icmp_handle = Some(icmp_handle);
    let mut tcp_handle = Some(tcp_handle);

    loop {
        if let Some(handle) = icmp_handle.take() {
            if handle.is_finished() {
                match handle.join() {
                    Ok(true) => {
                        log.warn(format!("{ip} found by ping!"));
                        return true;
                    }
                    _ => {}
                }
            } else {
                // Put it back if not finished
                icmp_handle = Some(handle);
            }
        }

        if let Some(handle) = tcp_handle.take() {
            if handle.is_finished() {
                match handle.join() {
                    Ok(true) => {
                        log.warn(format!("{ip} found by TCP connection!"));
                        return true;
                    }
                    _ => {}
                }
            } else {
                // Put it back if not finished
                tcp_handle = Some(handle);
            }
        }

        // If both threads are done (handles consumed), neither found the host to be up
        if icmp_handle.is_none() && tcp_handle.is_none() {
            return false;
        }

        thread::sleep(Duration::from_millis(2));
    }
}

fn native_icmp_ping(ip: Ipv4Addr) -> bool {
    let output = std::process::Command::new("ping")
        .arg("-c")
        .arg("1")
        .arg("-W")
        .arg("1")
        .arg(ip.to_string())
        .output();

    if let Ok(output) = output {
        output.status.success()
    } else {
        false
    }
}

fn icmp_ping(ip: Ipv4Addr) -> bool {
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_icmp_ping() {
        let is_up = icmp_ping(Ipv4Addr::from_str("127.0.0.1").unwrap());
        assert!(is_up);
    }
}
