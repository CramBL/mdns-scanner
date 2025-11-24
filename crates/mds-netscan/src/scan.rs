use std::{
    io,
    net::{IpAddr, Ipv4Addr},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering},
        mpsc::Sender,
    },
};

use mds_config::timeouts::Timeouts;
use mds_ipinfo::IpInfo;
use mds_util::prelude::*;

pub(crate) fn scan_ip_range(
    tx_info: &Sender<IpInfo>,
    network: &mds_util::NetworkInterface,
    num_threads: usize,
    timeout_settings: Timeouts,
    ports: &[u16],
    scanned_hosts_count: &Arc<AtomicU32>,
    cancellation_token: &Arc<AtomicBool>,
) {
    let prefix_len = network.prefix();
    let host_range = network.host_range();
    let host_count = network.host_count();
    let network_addr = mds_util::get_network_address_from_prefix(network.ip(), network.prefix());
    let netmask = mds_util::prefix_to_netmask(prefix_len);
    let network_description = format!("{name} {network_addr}/{prefix_len}", name = network.name());

    let local_ip = network.ip();

    log::info!(
        "{DISCOVERED_PREFIX}Running IP scan for {network_description}, netmask={netmask}, range={start}-{end} ({host_count})",
        netmask = netmask,
        start = host_range.start,
        end = host_range.end.saturating_sub(1), // -1 as the range is not inclusive
    );

    let pool = threadpool::Builder::new()
        .thread_name(format!("scan_worker_{}", network.name()))
        .num_threads(num_threads)
        .build();
    let network_int = u32::from(network_addr);

    for i in host_range {
        if i % 32 == 0 && cancellation_token.load(Ordering::Relaxed) {
            return;
        }
        let maybe_host_counter_updater = if i % 16 == 0 {
            Some(Arc::clone(scanned_hosts_count))
        } else {
            None
        };
        let ip_int = network_int + i;
        let ip = Ipv4Addr::from(ip_int);

        pool.execute({
            let tx_info = tx_info.clone();
            let tcp_ports = ports.to_vec();
            move || {
                if let Some(host_up_info) = check_host_up(ip, &tcp_ports, timeout_settings) {
                    let mut ip_info = IpInfo::from_ip(IpAddr::V4(ip)).with_info(host_up_info);

                    if let Some(hostnames) = dns_reverse_lookup(local_ip, ip) {
                        ip_info.set_names(hostnames);
                    }
                    let _ = tx_info.send(ip_info);
                }
                if let Some(host_counter_updater) = maybe_host_counter_updater {
                    host_counter_updater.fetch_max(i, Ordering::Relaxed);
                }
                // TODO: Add option to do reverse DNS lookup for hosts that are not discoverable through a network scan
                // i.e. hosts that no ports open to TCP connections and do not respond to ICMP packets.
                // NOTE: important(!) to distinguish between hostnames retrieved in this manner from hosts that
                // were reachable through TCP/ICMP, doing a reverse DNS lookup on all addresses will retrieve hostnames
                // from the router cache that can be VERY(!) old. It also needs to be able to gracefully replace these entries
                // if a new host is up on a subsequent network scan, and has a hostname that is actually active on the network
            }
        });
    }
    if cancellation_token.load(Ordering::Relaxed) {
        return;
    }
    pool.join();
    scanned_hosts_count.store(host_count, Ordering::Relaxed);
    if cancellation_token.load(Ordering::Relaxed) {
        return;
    }
    log::info!("{SUCCESS_PREFIX}Completed IP scan for network {network_description}");
}

pub(crate) fn dns_reverse_lookup(local_ip: Ipv4Addr, ip: Ipv4Addr) -> Option<Vec<String>> {
    log::trace!("Performing DNS lookup of {ip}");

    let mut hostnames: Option<Vec<String>> = None;

    // Try standard DNS reverse lookup first
    match dns_lookup::lookup_addr(&ip.into()) {
        Ok(hostname) => {
            log::info!("{DISCOVERED_PREFIX}DNS lookup:  {ip:13} -> {hostname}");
            hostnames = Some(vec![hostname]);
        }
        Err(e) => {
            // Don't log if it was a lookup to the local IP
            if local_ip != ip {
                log::warn!("DNS lookup failed '{ip}': {e}. Trying with mDNS lookup...");
            }
        }
    };

    // We always attempt mdns lookup even if regular lookup succeeds
    match mds_dns_sd::lookup::mdns_reverse_lookup(ip) {
        Ok(Some(hostname)) => {
            log::info!("{DISCOVERED_PREFIX}mDNS lookup: {ip:13} -> {hostname}");
            if let Some(hostnames) = hostnames.as_mut() {
                hostnames.push(hostname);
            } else {
                hostnames = Some(vec![hostname])
            }
        }
        Ok(None) => (),
        Err(e) => {
            // Don't log if it was a lookup to the local IP
            if local_ip != ip {
                match e.kind() {
                    // Unix typically returns WouldBlock and windows returns TimedOut if there's no response
                    // within the timeout we set
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => {
                        log::debug!("No response to mDNS lookup of {ip}")
                    }
                    _ => log::error!("mDNS lookup failed {ip}: {e}"),
                }
            }
        }
    }

    hostnames
}

#[cfg(test)]
mod tests {
    use super::*;
    use mds_config::timeouts::Timeouts;
    use mds_util::NetworkInterface;
    use std::net::Ipv4Addr;
    use std::sync::mpsc;
    use std::sync::{Arc, atomic::AtomicBool};

    #[test]
    fn test_scan_ip_range_cancellation_token() {
        let network = NetworkInterface::new("eth0".to_string(), Ipv4Addr::new(192, 168, 1, 10), 24);
        let (tx, rx) = mpsc::channel();
        let cancellation_token = Arc::new(AtomicBool::new(true));
        let scanned_host_count = Arc::new(AtomicU32::new(0));

        // ACT
        scan_ip_range(
            &tx,
            &network,
            4,
            Timeouts::default(),
            &[80, 443],
            &scanned_host_count,
            &cancellation_token,
        );

        // The thread pool should not have been joined if the token was set to true,
        // so no messages should have been sent.
        assert!(
            rx.recv_timeout(std::time::Duration::from_millis(100))
                .is_err()
        );
    }
}
