use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddrV4, UdpSocket},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
    time::Duration,
};

use mds_config::timeouts::Timeouts;
use mds_ipinfo::IpInfo;
use mds_log::prelude::Logger;
use mds_util::prelude::is_host_up;
use parking_lot::Mutex;

pub(crate) fn scan_ip_range(
    log: &Logger,
    tx_info: &Sender<IpInfo>,
    network: &mds_util::NetworkInterface,
    num_threads: usize,
    timeout_settings: Timeouts,
    ports: &[u16],
    cancellation_token: &Arc<AtomicBool>,
) -> Option<Vec<IpInfo>> {
    let prefix_len = network.prefix();
    let host_range = mds_util::calc_network_host_range(prefix_len);
    let network_addr = mds_util::get_network_address_from_prefix(network.ip(), network.prefix());
    let netmask = mds_util::prefix_to_netmask(prefix_len);
    let network_description = format!("{name} {network_addr}/{prefix_len}", name = network.name());

    log.info(format!(
        "🔍 Running IP scan for {network_description}, netmask={netmask}, range={start}-{end}",
        netmask = netmask,
        start = host_range.start,
        end = host_range.end
    ));

    let mut discovered: Option<Vec<IpInfo>> = None;

    let pool = threadpool::Builder::new()
        .thread_name(format!("scan_worker_{}", network.name()))
        .num_threads(num_threads)
        .build();
    let network_int = u32::from(network_addr);

    let hostnames = Arc::new(Mutex::new(Vec::<IpInfo>::new()));

    for i in host_range {
        if i % 32 == 0 && cancellation_token.load(Ordering::Relaxed) {
            return None;
        }
        let ip_int = network_int + i;
        let ip = Ipv4Addr::from(ip_int);

        let hostnames = Arc::clone(&hostnames);
        let log = log.clone();

        pool.execute({
            let tx_info = tx_info.clone();
            let tcp_ports = ports.to_vec();
            move || {
                if let Some(reached_by) =
                    is_host_up(ip, &tcp_ports, Some(log.clone()), timeout_settings)
                {
                    let mut ip_info = IpInfo::from_ip(IpAddr::V4(ip)).reached_with(reached_by);

                    if let Some(hostnames) = dns_reverse_lookup(ip, &log) {
                        ip_info.set_names(hostnames);
                    }
                    hostnames.lock().push(ip_info.clone());
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
        return None;
    }
    pool.join();
    if cancellation_token.load(Ordering::Relaxed) {
        return None;
    }
    let mut hostnames = hostnames.lock();
    if !hostnames.is_empty() {
        discovered = Some(hostnames.drain(..).collect());
    }
    if cancellation_token.load(Ordering::Relaxed) {
        return None;
    }
    log.info(format!(
        "✅ Completed IP scan for network {network_description}"
    ));
    discovered
}

pub(crate) fn dns_reverse_lookup(ip: Ipv4Addr, log: &Logger) -> Option<Vec<String>> {
    log.debug(format!("Performing DNS lookup of {ip}"));

    let mut hostnames: Option<Vec<String>> = None;

    // Try standard DNS reverse lookup first using std::net::lookup_addr
    match dns_lookup::lookup_addr(&ip.into()) {
        Ok(hostname) => {
            log.info(format!("🔍 DNS lookup: {ip:13} -> {hostname}"));
            hostnames = Some(vec![hostname]);
        }
        Err(e) => {
            log.warn(format!(
                "DNS lookup with lookup_addr failed: {e}. Trying with mDNS reverse lookup..."
            ));
        }
    };

    // We always attempt mdns lookup even if regular lookup succeeds
    match mdns_reverse_lookup(ip) {
        Ok(Some(hostname)) => {
            if let Some(hostnames) = hostnames.as_mut() {
                hostnames.push(hostname);
            } else {
                hostnames = Some(vec![hostname])
            }
        }
        Ok(None) => (),
        Err(e) => log.error(format!("mDNS lookup failed '{ip}': {e}")),
    }

    hostnames
}

pub(crate) fn mdns_reverse_lookup(ip: Ipv4Addr) -> io::Result<Option<String>> {
    let query = mds_util::build_reverse_dns_query(ip);
    let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))?;

    socket.set_read_timeout(Some(Duration::from_millis(1000)))?;
    socket.send_to(&query, mds_util::constants::MDNS_SOCKET_ADDR)?;

    let mut buf = [0u8; 1500];
    let (len, _) = socket.recv_from(&mut buf)?;
    if let Ok(packet) = dns_parser::Packet::parse(&buf[..len]) {
        for answer in packet.answers {
            if let dns_parser::RData::PTR(name) = answer.data {
                let hostname = name.to_string();
                return Ok(Some(hostname));
            }
        }
    }
    Ok(None)
}
