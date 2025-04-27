use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddrV4, UdpSocket},
    sync::{
        Arc, Mutex,
        atomic::{self, AtomicBool},
        mpsc::Sender,
    },
    time::Duration,
};

use get_if_addrs::Ifv4Addr;
use threadpool::ThreadPool;

use crate::{constants, ip_info::IpInfo, log::logger::Logger, util};

pub struct NetworkScanner {
    stop_flag: Arc<AtomicBool>,
    tx_info: Sender<IpInfo>,
    known_hosts: Vec<IpAddr>,
    logger: Logger,
}

impl NetworkScanner {
    pub(crate) fn new(stop_flag: Arc<AtomicBool>, tx_info: Sender<IpInfo>, logger: Logger) -> Self {
        Self {
            stop_flag,
            tx_info,
            known_hosts: vec![],
            logger,
        }
    }

    pub(crate) fn run(&mut self) {
        loop {
            let networks = util::get_network_params();
            for iface in &networks {
                self.logger.info(format!(
                    "🔌 Interface: {:<15} IP: {}",
                    iface.name, iface.addr.ip
                ));
            }
            let mut scanner_handles = vec![];

            for ifv4 in networks {
                let log_clone = self.logger.clone();
                let stop_flag = Arc::clone(&self.stop_flag);
                let tx_info = self.tx_info.clone();
                let scanner_handle = std::thread::Builder::new()
                    .name(format!("{}_scan_ip_range", ifv4.addr.ip))
                    .spawn(move || scan_ip_range(log_clone, tx_info, ifv4.addr, &stop_flag))
                    .expect("Failed spawning network scanner thread");
                scanner_handles.push(scanner_handle);
            }

            // Process all handles until they're all done
            while !scanner_handles.is_empty() {
                let mut completed_handles = vec![];

                let mut i = 0;
                while i < scanner_handles.len() {
                    if scanner_handles[i].is_finished() {
                        let handle = scanner_handles.remove(i);
                        completed_handles.push(handle);
                    } else {
                        i += 1;
                    }
                }

                for handle in completed_handles {
                    if let Ok(Some(ip_infos)) = handle.join() {
                        for ipi in ip_infos {
                            self.known_hosts.push(ipi.ip());
                        }
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            self.logger.info("SCANNER RUN OVER!");
        }
    }
}

pub(crate) fn scan_ip_range(
    mut log: Logger,
    tx_info: Sender<IpInfo>,
    network: Ifv4Addr,
    scan_in_progress: &AtomicBool,
) -> Option<Vec<IpInfo>> {
    let prefix_len = util::count_netmask_bits(network.netmask);
    let host_range = util::calc_network_host_range(prefix_len);
    let network_addr = util::get_network_address(&network);
    let network_description = format!("{network_addr}/{prefix_len}");

    log.info(format!(
        "🔍 Starting IP scan for network {network_description}, netmask={netmask}, range={start}-{end}",
        netmask = network.netmask,
        start = host_range.start,
        end = host_range.end
    ));

    let mut discovered: Option<Vec<IpInfo>> = None;

    let pool = ThreadPool::new(10); // Limit to 10 concurrent threads
    let network_int = u32::from(network_addr);

    let hostnames = Arc::new(Mutex::new(Vec::<IpInfo>::new()));

    for i in host_range {
        let ip_int = network_int + i;
        let ip = Ipv4Addr::from(ip_int);

        let hostnames = Arc::clone(&hostnames);
        let log = log.clone();

        pool.execute({
            let tx_info = tx_info.clone();
            move || {
                if crate::host_up::is_host_up(log.clone(), ip) {
                    let mut ip_info = IpInfo::from_ip(IpAddr::V4(ip));
                    if let Some(hostnames) = dns_reverse_lookup(ip, log) {
                        ip_info.names = hostnames;
                    }
                    hostnames.lock().unwrap().push(ip_info.clone());
                    tx_info.send(ip_info).unwrap();
                }
            }
        });
    }

    pool.join();
    let mut hostnames = hostnames.lock().unwrap();
    if !hostnames.is_empty() {
        discovered = Some(hostnames.drain(..).collect());
    }

    scan_in_progress.store(false, atomic::Ordering::Relaxed);
    log.info(format!(
        "✅ Completed IP scan for network {network_description}"
    ));
    discovered
}

pub(crate) fn dns_reverse_lookup(ip: Ipv4Addr, mut log: Logger) -> Option<Vec<String>> {
    log.info(format!("Performing DNS lookup of {ip}"));

    let mut hostnames: Option<Vec<String>> = None;

    // Try standard DNS reverse lookup first using std::net::lookup_addr
    match dns_lookup::lookup_addr(&ip.into()) {
        Ok(hostname) => {
            log.info(format!("🔍 DNS reverse lookup: {ip} -> {hostname}"));
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
        Err(e) => log.error(format!("mDNS reverse lookup failed '{ip}': {e}")),
    }

    hostnames
}

pub(crate) fn mdns_reverse_lookup(ip: Ipv4Addr) -> io::Result<Option<String>> {
    let query = util::build_reverse_dns_query(ip);
    let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))?;

    socket.set_read_timeout(Some(Duration::from_millis(1000)))?;
    socket.send_to(&query, constants::MDNS_SOCKET_ADDR)?;

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
