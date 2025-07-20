use mds_config::timeouts::Timeouts;
use mds_log::prelude::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use crate::ping;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReachedBy {
    Port(u16),
    EchoReply,
    Mdns,
}

pub fn is_host_up(
    ip: Ipv4Addr,
    ports: &[u16],
    mut log: Option<Logger>,
    timeouts: Timeouts,
) -> Option<ReachedBy> {
    let max_total_wait = timeouts.ip_check();
    if let Some(l) = &mut log {
        l.trace(format!("Checking if a host is up at {ip}"));
    }
    thread::scope(|scope| {
        let ping_timeout = timeouts.ping();
        let tcp_port_timeout = timeouts.tcp_port();

        let icmp_handle = scope.spawn(move || ping::icmp_ping(ip, ping_timeout));
        let tcp_handle = scope.spawn(move || up_by_tcp(ip, ports, tcp_port_timeout));
        let mut icmp_handle = Some(icmp_handle);
        let mut tcp_handle = Some(tcp_handle);

        let now = Instant::now();

        while now.elapsed() < max_total_wait {
            // Check if ICMP thread has finished and get its result
            if let Some(handle) = icmp_handle.take_if(|h| h.is_finished()) {
                if let Ok(Some(reached_in)) = handle.join() {
                    if let Some(l) = &mut log {
                        l.debug(format!("{ip} found with ping in {reached_in:.02?}"));
                    }
                    return Some(ReachedBy::EchoReply);
                }
            }

            // Check if TCP thread has finished and get its result
            if let Some(handle) = tcp_handle.take_if(|h| h.is_finished()) {
                if let Ok(Some(p)) = handle.join() {
                    if let Some(l) = &mut log {
                        l.debug(format!("{ip} found with TCP connection on port {p}"));
                    }
                    return Some(ReachedBy::Port(p));
                }
            }

            // If both threads are done (handles consumed), neither found the host to be up
            if icmp_handle.is_none() && tcp_handle.is_none() {
                return None;
            }

            thread::sleep(Duration::from_millis(2));
        }

        if let Some(l) = &mut log {
            l.error(format!("Exceeded max waiting time {max_total_wait:?} while waiting for ping or TCP connection to determine if host {ip} is up"));
        }
        None // Host is not up within the timeout
    })
}

pub fn up_by_tcp(ip: Ipv4Addr, ports: &[u16], port_timeout: Duration) -> Option<u16> {
    for port in ports {
        let socket_addr = SocketAddr::new(IpAddr::V4(ip), *port);
        if TcpStream::connect_timeout(&socket_addr, port_timeout).is_ok() {
            return Some(*port);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::prelude::IP_TEST_NET_1_UNREACHABLE;

    use super::*;
    use std::net::Ipv4Addr;

    const SSH_PORT: u16 = 22;
    const HTTP_PORT: u16 = 80;
    const HTTPS_PORT: u16 = 443;
    const SCAN_PORTS: &[u16] = &[SSH_PORT, HTTP_PORT, HTTPS_PORT];

    /// This IP is reserved for documentation (TEST-NET-1) and should never be reachable.
    /// This is the most reliable test case for a "host down" scenario.
    #[test]
    fn test_host_is_down_for_unreachable_ip() {
        assert!(
            is_host_up(
                IP_TEST_NET_1_UNREACHABLE,
                SCAN_PORTS,
                None,
                Timeouts::default()
            )
            .is_none(),
            "A documentation IP should always be down."
        );
    }

    #[test]
    fn test_localhost_is_up() {
        let ip = Ipv4Addr::new(127, 0, 0, 1);
        assert!(
            is_host_up(ip, SCAN_PORTS, None, Timeouts::default()).is_some(),
            "Localhost should be considered up."
        );
    }
}
