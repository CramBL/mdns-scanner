use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::thread;
use std::time::Duration;

use crate::log::logger::Logger;

const SSH_PORT: u16 = 22;
const HTTP_PORT: u16 = 80;
const HTTPS_PORT: u16 = 443;
const SCAN_PORTS: &[u16] = &[SSH_PORT, HTTP_PORT, HTTPS_PORT];

pub(crate) fn is_host_up(mut log: Logger, ip: Ipv4Addr) -> bool {
    log.trace(format!("Checking if a host is up at {ip}"));
    let icmp_handle = thread::spawn({
        let ip = ip.clone();
        move || crate::ping::icmp_ping(ip)
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
                        log.debug(format!("{ip} found with ping"));
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
                        log.debug(format!("{ip} found with TCP connection"));
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
