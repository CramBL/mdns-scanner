use std::{
    cmp,
    sync::{
        Arc,
        atomic::{self, AtomicBool, Ordering},
        mpsc::Sender,
    },
    thread::{self, JoinHandle},
    time::{self, Duration, Instant},
};

use mds_config::AppConfig;
use parking_lot::RwLock;

use mds_ipinfo::IpInfo;
use mds_log::prelude::*;
use mds_util::{refresh::RefreshListener, resource_scaling::HostResources};

mod scan;

pub struct NetworkScanner {
    stop_flag: Arc<AtomicBool>,
    tx_info: Sender<IpInfo>,
    logger: Logger,
    cfg: Arc<RwLock<AppConfig>>,
    refresh_listener: RefreshListener,
    host_resources: HostResources,
}

impl NetworkScanner {
    const MIN_THREADS_PER_SCAN: u16 = 10;

    pub fn new(
        stop_flag: Arc<AtomicBool>,
        tx_info: Sender<IpInfo>,
        logger: Logger,
        cfg: Arc<RwLock<AppConfig>>,
        refresh_listener: RefreshListener,
    ) -> Self {
        Self {
            stop_flag,
            tx_info,
            logger,
            cfg,
            refresh_listener,
            host_resources: HostResources::default(),
        }
    }

    fn should_ignore_interface(&self, interface_name: &str) -> bool {
        self.cfg
            .write()
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

    pub fn threads_per_scan(&mut self, num_network_interfaces: usize) -> u16 {
        let max_threads = match self.cfg.read().scan_thread_count() {
            mds_config::scan::IoThreads::Dynamic => self.host_resources.max_threads(),
            mds_config::scan::IoThreads::Fixed(count) => count,
        };
        cmp::max(
            Self::MIN_THREADS_PER_SCAN,
            max_threads / num_network_interfaces as u16,
        )
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

            let mut scanner_handles: Vec<JoinHandle<()>> = vec![];
            let threads_per_scan = self.threads_per_scan(network_interfaces_to_scan.len());
            let num_iface = network_interfaces_to_scan.len();
            if num_iface == 1 {
                self.logger.debug(format!(
                    "Network scan will use at most {threads_per_scan} I/O threads"
                ));
            } else {
                let threads_per_iface = threads_per_scan / num_iface as u16;
                self.logger.debug(format!(
                    "Network scan will use at most {threads_per_scan} I/O threads across {num_iface} interfaces ({threads_per_iface}/interface)"
                ));
            }

            let timeout_settings = self.cfg.read().timeout_settings();
            let scanner_cancellation = Arc::new(AtomicBool::new(false));
            let tcp_ports = self.cfg.read().scan_tcp_ports().clone();
            for ifv4 in network_interfaces_to_scan {
                let log_clone = self.logger.clone();
                let tx_info = self.tx_info.clone();
                let scanner_handle: thread::JoinHandle<()> = thread::Builder::new()
                    .name(format!("{}_scan_ip_range", ifv4.name()))
                    .spawn({
                        let scan_ports = tcp_ports.clone();
                        let cancellation_token = Arc::clone(&scanner_cancellation);
                        move || {
                            scan::scan_ip_range(
                                &log_clone,
                                &tx_info,
                                &ifv4,
                                threads_per_scan as usize,
                                timeout_settings,
                                &scan_ports,
                                &cancellation_token,
                            )
                        }
                    })
                    .expect("Failed spawning network scanner thread");
                scanner_handles.push(scanner_handle);
            }

            // Process all handles until they're all done
            while !scanner_handles.is_empty() && !self.refresh_listener.peek() {
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
                    if self.stop_flag.load(atomic::Ordering::Relaxed) {
                        break;
                    }
                    if let Err(e) = handle.join() {
                        if !self.stop_flag.load(atomic::Ordering::Relaxed) {
                            self.logger.error(format!("{e:?}"));
                        }
                    }
                }

                thread::sleep(time::Duration::from_millis(5));
            }
            if self.refresh_listener.do_refresh() {
                scanner_cancellation.store(true, Ordering::SeqCst);
                continue;
            }
            let scanner_time = now.elapsed();
            self.logger
                .info(format!("✅ Scanner run completed in {scanner_time:.02?}"));
            if scanner_time < Duration::from_secs(10) {
                thread::sleep(Duration::from_secs(5));
            }
        }
    }
}
