use mds_log::prelude::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::num::NonZeroU16;
use std::thread;
use std::time::{Duration, Instant};

use crate::ping;

const SSH_PORT: u16 = 22;
const HTTP_PORT: u16 = 80;
const HTTPS_PORT: u16 = 443;
const SCAN_PORTS: &[u16] = &[SSH_PORT, HTTP_PORT, HTTPS_PORT];

#[derive(Debug, Clone, Copy)]
pub struct TimeoutSettings {
    pub tcp_port_timeout_ms: NonZeroU16,
    pub ping_timeout_ms: NonZeroU16,
    pub ip_check_timeout_ms: NonZeroU16,
}

impl Default for TimeoutSettings {
    fn default() -> Self {
        Self {
            tcp_port_timeout_ms: NonZeroU16::new(100).unwrap(),
            ping_timeout_ms: NonZeroU16::new(100).unwrap(),
            ip_check_timeout_ms: NonZeroU16::new(100).unwrap(),
        }
    }
}

impl TimeoutSettings {
    pub fn tcp_port_timeout(&self) -> Duration {
        Duration::from_millis(self.tcp_port_timeout_ms.get().into())
    }

    pub fn ping_timeout(&self) -> Duration {
        Duration::from_millis(self.ping_timeout_ms.get().into())
    }

    pub fn ip_check_timeout(&self) -> Duration {
        Duration::from_millis(self.ip_check_timeout_ms.get().into())
    }
}

pub fn is_host_up(ip: Ipv4Addr, mut log: Option<Logger>, timeouts: TimeoutSettings) -> bool {
    let max_total_wait = timeouts.ip_check_timeout();
    if let Some(l) = &mut log {
        l.trace(format!("Checking if a host is up at {ip}"));
    }

    let ping_timeout = timeouts.ping_timeout();
    let mut icmp_handle = Some(thread::spawn(move || ping::icmp_ping(ip, ping_timeout)));
    let tcp_port_timeout = timeouts.tcp_port_timeout();
    let mut tcp_handle = Some(thread::spawn(move || up_by_tcp(ip, tcp_port_timeout)));

    let now = Instant::now();
    while now.elapsed() < max_total_wait {
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
    if let Some(l) = &mut log {
        l.error(format!("Exceeded max waiting time {max_total_wait:?} while waiting for ping or TCP connection to determine if host {ip} is up"));
    }

    false
}

fn up_by_tcp(ip: Ipv4Addr, port_timeout: Duration) -> bool {
    for port in SCAN_PORTS {
        let socket_addr = SocketAddr::new(IpAddr::V4(ip), *port);
        if TcpStream::connect_timeout(&socket_addr, port_timeout).is_ok() {
            return true;
        }
    }
    false
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
            !is_host_up(IP_TEST_NET_1_UNREACHABLE, None, TimeoutSettings::default()),
            "A documentation IP should always be down."
        );
    }

    #[test]
    fn test_localhost_is_up() {
        let ip = Ipv4Addr::new(127, 0, 0, 1);
        assert!(
            is_host_up(ip, None, TimeoutSettings::default()),
            "Localhost should be considered up."
        );
    }
}
