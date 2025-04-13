use crate::log::Logger;
use crate::util;
use get_if_addrs::Ifv4Addr;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::atomic::{self, AtomicBool};
use std::sync::{Arc, Mutex};

pub(crate) struct NetworkScan {
    pub(crate) network: Ifv4Addr,
    pub(crate) in_progress: Arc<AtomicBool>,
}

impl NetworkScan {
    pub fn new(network: Ifv4Addr) -> Self {
        Self {
            network,
            in_progress: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn set_in_progress(&self) {
        self.in_progress.store(true, atomic::Ordering::Relaxed);
    }

    pub fn in_progress(&self) -> bool {
        self.in_progress.load(atomic::Ordering::Relaxed)
    }

    pub fn get_in_progress_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.in_progress)
    }
}

pub(crate) fn scan_all_networks(
    log: &mut Logger,
    discovered_hosts: &Arc<Mutex<HashSet<IpAddr>>>,
    hostnames: &Arc<Mutex<HashMap<IpAddr, Vec<String>>>>,
    network_scans: &mut Vec<NetworkScan>,
) {
    let networks = util::get_network_params();

    for ifv4 in networks {
        let scan_in_progress_flag =
            if let Some(ns) = network_scans.iter().find(|ns| ns.network == ifv4) {
                if ns.in_progress() {
                    continue;
                } else {
                    ns.set_in_progress();
                    ns.get_in_progress_flag()
                }
            } else {
                let ns = NetworkScan::new(ifv4.clone());
                ns.set_in_progress();
                let scan_in_progress_flag = ns.get_in_progress_flag();
                network_scans.push(ns);
                scan_in_progress_flag
            };

        let log_clone = log.clone();
        let hosts_clone = Arc::clone(&discovered_hosts);
        let hostnames_clone = Arc::clone(&hostnames);

        std::thread::Builder::new()
            .name(format!("{}_scan_ip_range", ifv4.ip))
            .spawn(move || {
                crate::scan_ip::scan_ip_range(
                    log_clone,
                    ifv4,
                    hosts_clone,
                    hostnames_clone,
                    &scan_in_progress_flag,
                );
            })
            .unwrap();
    }
}
