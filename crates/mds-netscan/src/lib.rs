use std::{
    cmp, io,
    net::{IpAddr, Ipv4Addr, SocketAddrV4, UdpSocket},
    sync::{
        Arc,
        atomic::{self, AtomicBool},
        mpsc::Sender,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use mds_config::AppConfig;
use parking_lot::{Mutex, RwLock};

use mds_ipinfo::IpInfo;
use mds_log::prelude::*;
use mds_util::{host_up::TimeoutSettings, prelude::is_host_up};
use threadpool::ThreadPool;

pub struct NetworkScanner {
    stop_flag: Arc<AtomicBool>,
    tx_info: Sender<IpInfo>,
    known_hosts: Vec<IpAddr>,
    logger: Logger,
    cfg: Arc<RwLock<AppConfig>>,
}

impl NetworkScanner {
    const MAX_THREADS_PER_SCAN: usize = 100;
    const MIN_THREADS_PER_SCAN: usize = 10;

    pub fn new(
        stop_flag: Arc<AtomicBool>,
        tx_info: Sender<IpInfo>,
        logger: Logger,
        cfg: Arc<RwLock<AppConfig>>,
    ) -> Self {
        Self {
            stop_flag,
            tx_info,
            known_hosts: vec![],
            logger,
            cfg,
        }
    }

    fn should_ignore_interface(&self, interface_name: &str) -> bool {
        self.cfg
            .read()
            .iface_ignore_regex()
            .iter()
            .any(|pattern| pattern.is_match(interface_name))
    }

    fn get_network_interfaces(&self) -> Vec<mds_util::NetworkInterface> {
        let mut network_interfaces =
            mds_util::get_network_interfaces(self.cfg.read().iface_include_docker());
        network_interfaces.retain(|n| {
            if self.should_ignore_interface(n.name()) {
                self.logger.debug(format!(
                    "IGNORING: 🔌 Interface: {:<15} IP: {}",
                    n.name(),
                    n.ip()
                ));
                false
            } else {
                self.logger
                    .info(format!("🔌 Interface: {:<15} IP: {}", n.name(), n.ip()));
                true
            }
        });
        network_interfaces
    }

    pub fn spawn(mut self) {
        std::thread::Builder::new()
            .name("network_scanner".into())
            .spawn(move || {
                self.run();
            })
            .expect("Failed spawning network scanner thread");
    }

    pub fn run(&mut self) {
        while !self.stop_flag.load(atomic::Ordering::SeqCst) {
            let now = Instant::now();
            let network_interfaces_to_scan = self.get_network_interfaces();
            if network_interfaces_to_scan.is_empty() {
                self.logger.warn("No network interfaces to scan...");
                std::thread::sleep(Duration::from_secs(5));
                continue;
            }

            let mut scanner_handles: Vec<JoinHandle<Option<Vec<IpInfo>>>> = vec![];
            let threads_per_scan = cmp::max(
                Self::MIN_THREADS_PER_SCAN,
                Self::MAX_THREADS_PER_SCAN / network_interfaces_to_scan.len(),
            );
            self.logger.debug(format!(
                "Scanner threads will use at most {threads_per_scan} threads each"
            ));

            let timeout_settings = self.cfg.read().timeout_settings();
            for ifv4 in network_interfaces_to_scan {
                let log_clone = self.logger.clone();
                let tx_info = self.tx_info.clone();
                let scanner_handle: std::thread::JoinHandle<Option<Vec<IpInfo>>> =
                    std::thread::Builder::new()
                        .name(format!("{}_scan_ip_range", ifv4.name()))
                        .spawn(move || {
                            scan_ip_range(
                                &log_clone,
                                &tx_info,
                                &ifv4,
                                threads_per_scan,
                                timeout_settings,
                            )
                        })
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
                    if self.stop_flag.load(atomic::Ordering::SeqCst) {
                        break;
                    }
                    match handle.join() {
                        Ok(res) => {
                            if let Some(ip_infos) = res {
                                for ipi in ip_infos {
                                    self.known_hosts.push(ipi.ip());
                                }
                            }
                        }
                        Err(e) => {
                            if !self.stop_flag.load(atomic::Ordering::SeqCst) {
                                self.logger.error(format!("{e:?}"));
                            }
                        }
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            let scanner_time = now.elapsed();
            self.logger
                .info(format!("✅ Scanner run completed in {scanner_time:.02?}"));
            if scanner_time < Duration::from_secs(10) {
                std::thread::sleep(Duration::from_secs(5));
            }
        }
    }
}

pub(crate) fn scan_ip_range(
    log: &Logger,
    tx_info: &Sender<IpInfo>,
    network: &mds_util::NetworkInterface,
    num_threads: usize,
    timeout_settings: TimeoutSettings,
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

    let pool = ThreadPool::new(num_threads);
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
                if is_host_up(ip, Some(log.clone()), timeout_settings) {
                    let mut ip_info = IpInfo::from_ip(IpAddr::V4(ip));
                    if let Some(hostnames) = dns_reverse_lookup(ip, &log) {
                        ip_info.set_names(hostnames);
                    }
                    hostnames.lock().push(ip_info.clone());
                    let _ = tx_info.send(ip_info);
                }
            }
        });
    }

    pool.join();
    let mut hostnames = hostnames.lock();
    if !hostnames.is_empty() {
        discovered = Some(hostnames.drain(..).collect());
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
