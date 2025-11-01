use std::{
    sync::{
        Arc,
        atomic::{self, AtomicBool, AtomicU32, Ordering},
        mpsc::Sender,
    },
    thread::{self, JoinHandle},
    time::{self, Duration, Instant},
};

use mds_config::shared_config::SharedConfig;

use mds_ipinfo::IpInfo;
use mds_util::prelude::*;
use mds_util::{refresh::RefreshListener, resource_scaling::HostResources};
use smallvec::{SmallVec, smallvec};

use crate::progress::ScannerProgress;

pub mod progress;
mod scan;

pub struct NetworkScanner {
    stop_flag: Arc<AtomicBool>,
    tx_info: Sender<IpInfo>,
    cfg: SharedConfig,
    refresh_listener: RefreshListener,
    host_resources: HostResources,
    scanned_hosts_counts: SmallVec<[Arc<AtomicU32>; 3]>,
    scanner_progress: ScannerProgress,
}

impl NetworkScanner {
    const MIN_THREADS_PER_SCAN: u16 = 10;

    pub fn new(
        stop_flag: Arc<AtomicBool>,
        tx_info: Sender<IpInfo>,
        cfg: SharedConfig,
        refresh_listener: RefreshListener,
    ) -> Self {
        Self {
            stop_flag,
            tx_info,
            cfg,
            refresh_listener,
            host_resources: HostResources::default(),
            scanned_hosts_counts: smallvec![],
            scanner_progress: ScannerProgress::default(),
        }
    }

    #[must_use]
    pub fn spawn(mut self) -> ScannerProgress {
        let scanner_progress = self.scanner_progress.clone();
        std::thread::Builder::new()
            .name("network_scanner".into())
            .spawn(move || {
                self.run();
            })
            .expect("Failed spawning network scanner thread");
        scanner_progress
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
                log::debug!(
                    "IGNORING: {NETWORK_INTERFACE}Interface: {:<15} IP: {}",
                    n.name(),
                    n.ip()
                );
                false
            } else {
                log::info!(
                    "{NETWORK_INTERFACE}Interface: {:<15} IP: {}",
                    n.name(),
                    n.ip()
                );
                true
            }
        });
        network_interfaces
    }

    fn threads_per_scan(&mut self, num_network_interfaces: usize) -> u16 {
        let max_threads = match self.cfg.read().scan_io_threads() {
            mds_config::scan::IoThreads::Dynamic => self.host_resources.max_threads(),
            mds_config::scan::IoThreads::Fixed(count) => count,
        };
        Self::MIN_THREADS_PER_SCAN.max(max_threads / num_network_interfaces as u16)
    }

    fn update_scanner_progress(&self, scanner_progress: &mut u32) {
        let new_scanner_progress = self
            .scanned_hosts_counts
            .iter()
            .fold(0, |cnt, ifv_cnt| cnt + ifv_cnt.load(Ordering::Relaxed));
        if *scanner_progress != new_scanner_progress {
            debug_assert!(*scanner_progress < new_scanner_progress);
            *scanner_progress = new_scanner_progress;
            self.scanner_progress.update(*scanner_progress);
        }
    }

    /// Process all handles until they're all done
    fn run_progress_polling(
        &self,
        total_host_count: u32,
        mut scanner_handles: Vec<JoinHandle<()>>,
    ) {
        let mut scanner_progress: u32 = 0;
        self.scanner_progress.start(total_host_count);
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
                        log::error!("{e:?}");
                    }
                }
            }

            self.update_scanner_progress(&mut scanner_progress);

            thread::sleep(time::Duration::from_millis(5));
        }
    }

    fn run(&mut self) {
        while !self.stop_flag.load(atomic::Ordering::SeqCst) {
            let now = Instant::now();
            let network_interfaces_to_scan = self.get_network_interfaces();
            if network_interfaces_to_scan.is_empty() {
                log::warn!("No network interfaces to scan...");

                std::thread::sleep(Duration::from_secs(5));
                continue;
            }

            let mut scanner_handles: Vec<JoinHandle<()>> = vec![];
            let threads_per_scan = self.threads_per_scan(network_interfaces_to_scan.len());
            let num_iface = network_interfaces_to_scan.len();
            if num_iface == 1 {
                log::debug!("Network scan will use at most {threads_per_scan} I/O threads");
            } else {
                let threads_per_iface = threads_per_scan / num_iface as u16;
                log::debug!(
                    "Network scan will use at most {threads_per_scan} I/O threads across {num_iface} interfaces ({threads_per_iface}/interface)"
                );
            }

            let total_host_count = network_interfaces_to_scan
                .iter()
                .fold(0, |cnt, ifv| cnt + ifv.host_count());

            log::info!(
                "Scanning {num_iface} interface{plurality} for {total_host_count} potential hosts",
                plurality = if num_iface == 1 { "" } else { "s" }
            );

            let timeout_settings = self.cfg.read().timeout_settings();
            let scanner_cancellation = Arc::new(AtomicBool::new(false));
            let tcp_ports = self.cfg.read().scan_tcp_ports().clone();
            self.scanned_hosts_counts.clear();
            for ifv4 in network_interfaces_to_scan {
                let scanned_hosts_count = Arc::new(AtomicU32::new(0));
                self.scanned_hosts_counts
                    .push(Arc::clone(&scanned_hosts_count));
                let tx_info = self.tx_info.clone();
                let scanner_handle: thread::JoinHandle<()> = thread::Builder::new()
                    .name(format!("{}_scan_ip_range", ifv4.name()))
                    .spawn({
                        let scan_ports = tcp_ports.clone();
                        let cancellation_token = Arc::clone(&scanner_cancellation);
                        move || {
                            scan::scan_ip_range(
                                &tx_info,
                                &ifv4,
                                threads_per_scan as usize,
                                timeout_settings,
                                &scan_ports,
                                &scanned_hosts_count,
                                &cancellation_token,
                            )
                        }
                    })
                    .expect("Failed spawning network scanner thread");
                scanner_handles.push(scanner_handle);
            }

            self.run_progress_polling(total_host_count, scanner_handles);

            if self.refresh_listener.do_refresh() {
                scanner_cancellation.store(true, Ordering::SeqCst);
                continue;
            }

            self.scanner_progress.finish();
            let scanner_time = now.elapsed();
            log::info!("{SUCCESS_PREFIX}Scanner run completed in {scanner_time:.02?}");
            if scanner_time < Duration::from_secs(10) {
                thread::sleep(Duration::from_secs(5));
            }
        }
    }
}
