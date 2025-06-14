use mds_log::prelude::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::thread;
use std::time::Duration;

use crate::ping;

const SSH_PORT: u16 = 22;
const HTTP_PORT: u16 = 80;
const HTTPS_PORT: u16 = 443;
const SCAN_PORTS: &[u16] = &[SSH_PORT, HTTP_PORT, HTTPS_PORT];

fn up_by_tcp(ip: Ipv4Addr) -> bool {
    for port in SCAN_PORTS {
        let socket_addr = SocketAddr::new(IpAddr::V4(ip), *port);
        if TcpStream::connect_timeout(&socket_addr, Duration::from_millis(100)).is_ok() {
            return true;
        }
    }
    false
}

pub fn is_host_up(ip: Ipv4Addr, mut log: Option<Logger>) -> bool {
    if let Some(l) = &mut log {
        l.trace(format!("Checking if a host is up at {ip}"));
    }

    let mut icmp_handle = Some(thread::spawn(move || ping::icmp_ping(ip)));
    let mut tcp_handle = Some(thread::spawn(move || up_by_tcp(ip)));

    loop {
        if let Some(handle) = icmp_handle.take_if(|h| h.is_finished()) {
            if matches!(handle.join(), Ok(true)) {
                if let Some(l) = &mut log {
                    l.debug(format!("{ip} found with ping"));
                }
                return true;
            }
        }

        if let Some(handle) = tcp_handle.take_if(|h| h.is_finished()) {
            if matches!(handle.join(), Ok(true)) {
                if let Some(l) = &mut log {
                    l.debug(format!("{ip} found with TCP connection"));
                }
                return true;
            }
        }

        // If both threads are done (handles consumed), neither found the host to be up
        if icmp_handle.is_none() && tcp_handle.is_none() {
            return false;
        }

        thread::sleep(Duration::from_millis(2));
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::IP_TEST_NET_1_UNREACHABLE;

    use super::*;
    use std::net::Ipv4Addr;

    /// This IP is reserved for documentation (TEST-NET-1) and should never be reachable.
    /// This is the most reliable test case for a "host down" scenario.
    #[test]
    fn test_host_is_down_for_unreachable_ip() {
        assert!(
            !is_host_up(IP_TEST_NET_1_UNREACHABLE, None),
            "A documentation IP should always be down."
        );
    }

    #[test]
    fn test_localhost_is_up() {
        let ip = Ipv4Addr::new(127, 0, 0, 1);
        assert!(is_host_up(ip, None), "Localhost should be considered up.");
    }
}
