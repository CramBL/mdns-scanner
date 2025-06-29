use std::{
    net::{IpAddr, Ipv4Addr},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
};

use mds_config::timeouts::Timeouts;
use mds_ipinfo::IpInfo;
use mds_log::prelude::Logger;
use mds_util::prelude::is_host_up;

pub(crate) fn scan_ip_range(
    log: &Logger,
    tx_info: &Sender<IpInfo>,
    network: &mds_util::NetworkInterface,
    num_threads: usize,
    timeout_settings: Timeouts,
    ports: &[u16],
    cancellation_token: &Arc<AtomicBool>,
) {
    let prefix_len = network.prefix();
    let host_range = mds_util::calc_network_host_range(prefix_len);
    let network_addr = mds_util::get_network_address_from_prefix(network.ip(), network.prefix());
    let netmask = mds_util::prefix_to_netmask(prefix_len);
    let network_description = format!("{name} {network_addr}/{prefix_len}", name = network.name());

    let local_ip = network.ip();

    log.info(format!(
        "🔍 Running IP scan for {network_description}, netmask={netmask}, range={start}-{end}",
        netmask = netmask,
        start = host_range.start,
        end = host_range.end
    ));

    let pool = threadpool::Builder::new()
        .thread_name(format!("scan_worker_{}", network.name()))
        .num_threads(num_threads)
        .build();
    let network_int = u32::from(network_addr);

    for i in host_range {
        if i % 32 == 0 && cancellation_token.load(Ordering::Relaxed) {
            return;
        }
        let ip_int = network_int + i;
        let ip = Ipv4Addr::from(ip_int);

        let log = log.clone();

        pool.execute({
            let tx_info = tx_info.clone();
            let tcp_ports = ports.to_vec();
            move || {
                if let Some(reached_by) =
                    is_host_up(ip, &tcp_ports, Some(log.clone()), timeout_settings)
                {
                    let mut ip_info = IpInfo::from_ip(IpAddr::V4(ip)).reached_with(reached_by);

                    if let Some(hostnames) = dns_reverse_lookup(&log, local_ip, ip) {
                        ip_info.set_names(hostnames);
                    }
                    let _ = tx_info.send(ip_info);
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
    if cancellation_token.load(Ordering::SeqCst) {
        return;
    }
    pool.join();
    if cancellation_token.load(Ordering::Relaxed) {
        return;
    }
    log.info(format!(
        "✅ Completed IP scan for network {network_description}"
    ));
}

pub(crate) fn dns_reverse_lookup(
    log: &Logger,
    local_ip: Ipv4Addr,
    ip: Ipv4Addr,
) -> Option<Vec<String>> {
    log.debug(format!("Performing DNS lookup of {ip}"));

    let mut hostnames: Option<Vec<String>> = None;

    // Try standard DNS reverse lookup first
    match dns_lookup::lookup_addr(&ip.into()) {
        Ok(hostname) => {
            log.info(format!("🔍 DNS lookup: {ip:13} -> {hostname}"));
            hostnames = Some(vec![hostname]);
        }
        Err(e) => {
            log.warn(format!(
                "DNS lookup failed '{ip}': {e}. Trying with mDNS reverse lookup..."
            ));
        }
    };

    // We always attempt mdns lookup even if regular lookup succeeds
    match mds_dns_sd::lookup::mdns_reverse_lookup(log, ip) {
        Ok(Some(hostname)) => {
            log.info(format!("🔍 mDNS lookup: {ip:13} -> {hostname}"));
            if let Some(hostnames) = hostnames.as_mut() {
                hostnames.push(hostname);
            } else {
                hostnames = Some(vec![hostname])
            }
        }
        Ok(None) => (),
        Err(e) => {
            // Don't log error if it was an mDNS lookup to the local IP
            if local_ip != ip {
                log.error(format!("mDNS lookup failed '{ip}': {e}"));
            }
        }
    }

    hostnames
}

#[cfg(test)]
mod tests {
    use super::*;
    use mds_config::timeouts::Timeouts;
    use mds_log::prelude::Logger;
    use mds_util::NetworkInterface;
    use std::net::Ipv4Addr;
    use std::sync::mpsc;
    use std::sync::{Arc, atomic::AtomicBool};

    #[test]
    fn test_scan_ip_range_cancellation_token() {
        let network = NetworkInterface::new("eth0".to_string(), Ipv4Addr::new(192, 168, 1, 10), 24);
        let (log_tx, _log_rx) = mpsc::channel();
        let log = Logger::new(log_tx, mds_log::LogLevel::Trace);
        let (tx, rx) = mpsc::channel();
        let cancellation_token = Arc::new(AtomicBool::new(true));

        // ACT
        scan_ip_range(
            &log,
            &tx,
            &network,
            4,
            Timeouts::default(),
            &[80, 443],
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
