use dns_parser::RData;
use get_if_addrs::Ifv4Addr;

use crate::log::Logger;
use crate::{constants, util};
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4, UdpSocket};
use std::sync::atomic::{self, AtomicBool};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn insert_discovered_if_new(
    log: &mut Logger,
    discovered_hosts: &Mutex<HashSet<IpAddr>>,
    ip: Ipv4Addr,
) {
    let mut discovered = discovered_hosts.lock().unwrap();
    if !discovered.contains(&IpAddr::V4(ip)) {
        log.info(format!("🖥️ Found active host: {ip}"));
        discovered.insert(IpAddr::V4(ip));
    }
}

fn calc_network_host_range(prefix_len: u8) -> std::ops::Range<u32> {
    let host_bits = 32 - prefix_len;
    let host_count = 2u32.pow(host_bits as u32);
    // Skip network address (0) and broadcast address (host_count - 1)
    let host_range = 1..host_count - 1;
    host_range
}

use threadpool::ThreadPool;

pub(crate) fn scan_ip_range(
    mut log: Logger,
    network: Ifv4Addr,
    discovered_hosts: Arc<Mutex<HashSet<IpAddr>>>,
    hostnames: Arc<Mutex<HashMap<IpAddr, Vec<String>>>>,
    scan_in_progress: &AtomicBool,
) {
    let prefix_len = util::count_netmask_bits(network.netmask);
    let host_range = calc_network_host_range(prefix_len);
    let network_addr = util::get_network_address(&network);
    let network_description = format!("{network_addr}/{prefix_len}");

    log.info(format!(
        "🔍 Starting IP scan for network {network_description}, netmask={netmask}, range={start}-{end}",
        netmask = network.netmask,
        start = host_range.start,
        end = host_range.end
    ));

    let pool = ThreadPool::new(10); // Limit to 10 concurrent threads
    let network_int = u32::from(network_addr);

    for i in host_range {
        let ip_int = network_int + i;
        let ip = Ipv4Addr::from(ip_int);

        let discovered_hosts = Arc::clone(&discovered_hosts);
        let hostnames = Arc::clone(&hostnames);
        let mut log = log.clone();

        pool.execute(move || {
            if crate::host_up::is_host_up(log.clone(), ip) {
                insert_discovered_if_new(&mut log, &discovered_hosts, ip);
                dns_reverse_lookup(ip, hostnames, log);
            }
        });
    }

    pool.join();

    scan_in_progress.store(false, atomic::Ordering::Relaxed);
    log.info(format!(
        "✅ Completed IP scan for network {network_description}"
    ));
}

pub(crate) fn mdns_reverse_lookup(
    ip: Ipv4Addr,
    hostnames: Arc<Mutex<HashMap<IpAddr, Vec<String>>>>,
    mut log: Logger,
) {
    let query = util::build_reverse_dns_query(ip);
    let socket = match UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)) {
        Ok(s) => s,
        Err(e) => {
            log.warn(format!(
                "Failed to create socket for mDNS reverse lookup: {e}"
            ));
            return;
        }
    };

    // Set timeout for the socket
    if let Err(e) = socket.set_read_timeout(Some(Duration::from_millis(500))) {
        log.error(format!("Failed to set socket timeout: {}", e));
        return;
    }

    // Send the query to the mDNS multicast address
    if let Err(e) = socket.send_to(&query, constants::MDNS_SOCKET_ADDR) {
        log.error(format!("Failed to send mDNS reverse lookup: {}", e));
        return;
    }

    // Wait for responses
    let mut buf = [0u8; 1500];
    let mut received_responses = false;

    // Try a few times as mDNS responses might take time
    for _ in 0..3 {
        match socket.recv_from(&mut buf) {
            Ok((len, _)) => {
                if let Ok(packet) = dns_parser::Packet::parse(&buf[..len]) {
                    received_responses = true;

                    for answer in packet.answers {
                        if let dns_parser::RData::PTR(name) = answer.data {
                            let hostname = name.to_string();
                            log.info(format!("🔍 mDNS reverse lookup: {} -> {}", ip, hostname));
                            insert_ip_hostname_if_new(&hostnames, ip, hostname);
                        }
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Timeout occurred, which is expected
                break;
            }
            Err(e) => {
                log.error(format!("Error receiving mDNS response: {}", e));
                break;
            }
        }

        if received_responses {
            break;
        }

        // Short delay before trying again
        std::thread::sleep(Duration::from_millis(200));
    }
}

pub(crate) fn insert_ip_hostname_if_new(
    hostnames: &Mutex<HashMap<IpAddr, Vec<String>>>,
    ip: Ipv4Addr,
    hostname: String,
) {
    let mut hostnames_map = hostnames.lock().unwrap();
    let entry = hostnames_map.entry(ip.into()).or_default();
    if !entry.contains(&hostname) {
        entry.push(hostname);
    }
}

pub(crate) fn dns_reverse_lookup(
    ip: Ipv4Addr,
    hostnames: Arc<Mutex<HashMap<IpAddr, Vec<String>>>>,
    mut log: Logger,
) {
    log.info(format!("Performing DNS lookup of {ip}"));

    // Try standard DNS reverse lookup first using std::net::lookup_addr
    match dns_lookup::lookup_addr(&ip.into()) {
        Ok(hostname) => {
            log.info(format!("🔍 DNS reverse lookup: {ip} -> {hostname}"));
            insert_ip_hostname_if_new(&hostnames, ip, hostname);
        }
        Err(e) => {
            log.warn(format!(
                "DNS lookup with lookup_addr failed: {e}. Trying with mDNS reverse lookup..."
            ));
        }
    };

    // We always attempt mdns lookup even if regular lookup succeeds
    mdns_reverse_lookup(ip, hostnames, log);
}

pub(crate) fn extract_hostnames_from_mdns(
    packet: &dns_parser::Packet,
    source_ip: IpAddr,
    hostnames: Arc<Mutex<HashMap<IpAddr, Vec<String>>>>,
    log: &mut Logger,
) {
    let mut found_hostnames = Vec::new();

    // Look through answers for hostname information
    for answer in &packet.answers {
        // Check for A records that map hostnames to IPs
        if let RData::A(addr) = answer.data {
            let ip_addr = IpAddr::V4(addr.0);
            let hostname = answer.name.to_string();

            if !hostname.is_empty() && hostname != "localhost" {
                found_hostnames.push((ip_addr, hostname.clone()));
                log.info(format!(
                    "📝 Found hostname mapping: {hostname} -> {ip_addr}"
                ));
            }
        }

        // Check for PTR records that map IPs to hostnames
        if let RData::PTR(name) = &answer.data {
            let hostname = name.to_string();
            if !hostname.is_empty() && hostname != "localhost" {
                found_hostnames.push((source_ip, hostname.clone()));
                log.info(format!("📝 Found PTR record: {source_ip} -> {hostname}",));
            }
        }
    }

    // Also check additional records
    for additional in &packet.additional {
        if let RData::A(addr) = additional.data {
            let ip_addr = IpAddr::V4(addr.0);
            let hostname = additional.name.to_string();

            if !hostname.is_empty() && hostname != "localhost" {
                found_hostnames.push((ip_addr, hostname.clone()));
                log.info(format!(
                    "📝 Found additional hostname mapping: {hostname} -> {ip_addr}"
                ));
            }
        }
    }

    // Update the hostname map
    let mut hostnames_map = hostnames.lock().unwrap();
    for (ip, hostname) in found_hostnames {
        let entry = hostnames_map.entry(ip).or_default();
        if !entry.contains(&hostname) {
            entry.push(hostname);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::LogLevel;
    use std::{str::FromStr, sync::mpsc::channel};

    #[test]
    fn test_mdns_reverse_lookup() {
        let hostnames: Arc<Mutex<HashMap<IpAddr, Vec<String>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let (tx, rx) = channel();
        let logger = Logger::new(tx, LogLevel::Trace);

        mdns_reverse_lookup(
            Ipv4Addr::from_str("192.168.0.81").unwrap(),
            hostnames,
            logger,
        );

        while let Ok(m) = rx.recv_timeout(Duration::from_secs(5)) {
            eprintln!("{m:?}");
        }
    }
}
