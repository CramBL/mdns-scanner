use dns_parser::RData;
use get_if_addrs::Ifv4Addr;

use crate::ip_info::IpInfo;
use crate::log::Logger;
use crate::{constants, util};
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4, UdpSocket};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

pub(crate) fn scan_ip_range(
    network: Ifv4Addr,
    discovered_hosts: Arc<Mutex<HashSet<IpAddr>>>,
    hostnames: Arc<Mutex<HashMap<IpAddr, Vec<String>>>>,
    sender: mpsc::Sender<IpInfo>,
    mut log: Logger,
) {
    // Calculate CIDR prefix length from netmask
    let prefix_len = util::count_netmask_bits(network.netmask);

    // Calculate network address
    let network_addr = util::get_network_address(&network);
    log.trace(format!(
        "calculated network_addr={network_addr}, from: ip={ip}, netmask={netmask}",
        ip = network.ip,
        netmask = network.netmask
    ));

    log.info(format!(
        "🔍 Starting IP scan for network {network_addr}/{prefix_len}",
    ));

    // Calculate the number of hosts in this subnet
    let host_bits = 32 - prefix_len;
    let host_count = 2u32.pow(host_bits as u32);

    let network_int = u32::from(network_addr);

    // Scan each IP in the subnet
    for i in 1..host_count - 1 {
        // Skip network address (0) and broadcast address (host_count - 1)
        let ip_int = network_int + i;
        let ip = Ipv4Addr::from(ip_int);

        // Skip checking IPs we've already discovered
        {
            let discovered = discovered_hosts.lock().unwrap();
            if discovered.contains(&IpAddr::V4(ip)) {
                continue;
            }
        }

        if util::is_host_up(ip) {
            let ip_addr = IpAddr::V4(ip);
            log.info(format!("🖥️ Found active host: {ip}"));

            // Add to discovered hosts
            {
                let mut discovered = discovered_hosts.lock().unwrap();
                discovered.insert(ip_addr);
            }

            // Try to determine hostname
            let hostnames_clone = Arc::clone(&hostnames);
            let log_clone = log.clone();
            std::thread::Builder::new()
                .name(format!("{ip} dns_reverse_lookup"))
                .spawn(move || {
                    dns_reverse_lookup(ip, hostnames_clone, log_clone);
                })
                .expect("Failed to spawn dns_reverse_lookup thread");

            let mut ip_info = IpInfo::from_ip(ip_addr);

            // Check if we already have hostname information
            {
                let hostnames_map = hostnames.lock().unwrap();
                if let Some(host_names) = hostnames_map.get(&ip_addr) {
                    for hostname in host_names {
                        ip_info.names.push(hostname.clone());
                    }
                }
            }

            if let Err(e) = sender.send(ip_info) {
                log.error(format!("Failed to send IP scan results: {}", e));
            }
        }
    }

    log.info(format!(
        "✅ Completed IP scan for network {network_addr}/{prefix_len}"
    ));
}

pub(crate) fn dns_reverse_lookup(
    ip: Ipv4Addr,
    hostnames: Arc<Mutex<HashMap<IpAddr, Vec<String>>>>,
    mut log: Logger,
) {
    let ip_addr = IpAddr::V4(ip);

    // Check if we already have hostname information
    {
        let hostnames_map = hostnames.lock().unwrap();
        if hostnames_map.contains_key(&ip_addr) && !hostnames_map.get(&ip_addr).unwrap().is_empty()
        {
            log.debug(format!("Already have hostname information for ip: {ip}"));
            return;
        }
    }

    // Try standard DNS reverse lookup first using std::net::lookup_addr
    match dns_lookup::lookup_addr(&ip_addr) {
        Ok(hostname) => {
            log.info(format!("🔍 DNS reverse lookup: {ip} -> {hostname}"));
            let mut hostnames_map = hostnames.lock().unwrap();
            let entry = hostnames_map.entry(ip_addr).or_insert_with(Vec::new);
            if !entry.contains(&hostname) {
                entry.push(hostname);
            }
        }
        Err(e) => {
            log.warn(format!(
                "DNS lookup with lookup_addr failed: {e}. Trying with mDNS reverse lookup..."
            ));
            let query = util::build_reverse_dns_query(ip);
            let socket = match UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)) {
                Ok(s) => s,
                Err(e) => {
                    log.warn(format!(
                        "Failed to create socket for mDNS reverse lookup: {}",
                        e
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
                                    log.info(format!(
                                        "🔍 mDNS reverse lookup: {} -> {}",
                                        ip, hostname
                                    ));

                                    let mut hostnames_map = hostnames.lock().unwrap();
                                    let entry =
                                        hostnames_map.entry(ip_addr).or_insert_with(Vec::new);
                                    if !entry.contains(&hostname) {
                                        entry.push(hostname);
                                    }
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
    }
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
        let entry = hostnames_map.entry(ip).or_insert_with(Vec::new);
        if !entry.contains(&hostname) {
            entry.push(hostname);
        }
    }
}
