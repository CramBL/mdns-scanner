//! Ip info collector receives [IpInfo] and sends new or modified [IpInfo] to the TUI
use dns_sd_discoverer::DnsSdDiscoverer;
use hosts_up_checker::HostsUpChecker;

use mds_config::shared_config::SharedConfig;
use mds_dns_sd::prelude::*;
use mds_ipinfo::service::ServiceInstance;
use mds_ipinfo::{IpInfo, LastKnownStatus};
use mds_util::host_up::ReachedBy;
use mds_util::refresh::RefreshListener;

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};
use std::{io, thread};

mod dns_sd_discoverer;
mod hosts_up_checker;

pub fn spawn_collector(
    stop_flag: Arc<AtomicBool>,
    rx_from_scanners: Receiver<IpInfo>,
    tx_to_table_pane: Sender<CollectorUpdate>,
    cfg: SharedConfig,
    refresh_listener: RefreshListener,
) {
    let mut collector = IpInfoCollector::new(
        stop_flag,
        rx_from_scanners,
        tx_to_table_pane,
        cfg,
        refresh_listener,
    );
    thread::Builder::new()
        .name("ipinfo_collector".into())
        .spawn(move || {
            collector.run();
            collector
        })
        .expect("Failed spawning Ip info collector thread");
}

#[derive(Debug)]
pub enum CollectorUpdate {
    IpInfo(IpInfo),
    PacketSeen {
        ip: IpAddr,
        rtt: Option<Duration>,
    },
    Status((IpAddr, (LastKnownStatus, Option<Duration>))),
    /// Indicates that all information after this message is fresh, and all before it is stale
    Refresh,
}

struct IpInfoCollector {
    db: HashMap<IpAddr, IpInfo>,
    rx_info: Receiver<IpInfo>,
    tx_info: Sender<CollectorUpdate>,
    stop_flag: Arc<AtomicBool>,
    update_msgs: Vec<CollectorUpdate>,
    hosts_up_checker: HostsUpChecker,
    dns_sd_discoverer: DnsSdDiscoverer,
    cfg: SharedConfig,
    refresh_listener: RefreshListener,
}

impl IpInfoCollector {
    // How often to check for known hosts being up (time since last check)
    const HOST_UP_CHECK_INTERVAL_SECS: u8 = 16;
    const DNS_SD_DISCOVERY_INTERVAL_SECS: u8 = 30;

    fn new(
        stop_flag: Arc<AtomicBool>,
        rx_info: Receiver<IpInfo>,
        tx_info: Sender<CollectorUpdate>,
        cfg: SharedConfig,
        refresh_listener: RefreshListener,
    ) -> Self {
        let service_discovery_enabled = cfg.read().service_discovery_enabled();
        Self {
            db: HashMap::new(),
            rx_info,
            tx_info,
            stop_flag,
            update_msgs: vec![],
            hosts_up_checker: HostsUpChecker::new(
                Self::HOST_UP_CHECK_INTERVAL_SECS.into(),
                cfg.clone(),
            ),
            dns_sd_discoverer: DnsSdDiscoverer::new(
                Self::DNS_SD_DISCOVERY_INTERVAL_SECS.into(),
                service_discovery_enabled,
            ),
            cfg,
            refresh_listener,
        }
    }

    fn insert(&mut self, ip_info: IpInfo) {
        self.db.insert(ip_info.ip(), ip_info);
    }

    fn insert_or_update(&mut self, mut new_ip_info: IpInfo) {
        let new_ip = new_ip_info.ip();
        if let Some(old_ip_info) = self.db.get_mut(&new_ip) {
            if *old_ip_info != new_ip_info {
                let mut item_modified = false;
                for n in new_ip_info.names() {
                    if !old_ip_info.contains(n) {
                        old_ip_info.add_name(n.clone());
                        old_ip_info.sort_names();
                        item_modified = true;
                    }
                }
                for mut service in new_ip_info.drain_services() {
                    // Remove the service's hostname string if the service is available under
                    // the same hostname as the host machine, otherwise it is advertising under an
                    // independent mDNS hostname
                    service.remove_hostname_if_contained_in(old_ip_info.names());
                    if old_ip_info.update_with_service_instance(service) {
                        item_modified = true;
                    }
                }

                // Overwrite the existing "reached by" if it is mDNS and the new one isn't
                // because then we can check for its existence in the "host up checker"
                if let Some(old_rb) = old_ip_info.reached_by()
                    && old_rb == ReachedBy::Mdns
                    && let Some(new_rb) = new_ip_info.reached_by()
                    && new_rb != ReachedBy::Mdns
                {
                    old_ip_info.set_reached_by(new_rb);
                }

                old_ip_info.incr_seen_count();
                if item_modified {
                    self.update_msgs
                        .push(CollectorUpdate::IpInfo(old_ip_info.clone()));
                } else {
                    // It might be None if the new ip info is from a discovered service
                    let rtt: Option<Duration> = new_ip_info.rtt.map(|rtt| rtt.first);
                    self.update_msgs
                        .push(CollectorUpdate::PacketSeen { ip: new_ip, rtt });
                }
            }
        } else {
            new_ip_info.dedup_names();
            new_ip_info.sort_names();
            self.insert(new_ip_info.clone());
            self.update_msgs.push(CollectorUpdate::IpInfo(new_ip_info));
        }
    }

    fn poll_host_checker(&mut self, force_refresh: bool) {
        if force_refresh {
            self.hosts_up_checker.reset();
        } else if self.hosts_up_checker.is_time_to_run() {
            log::info!("Running status check for known hosts");
            self.hosts_up_checker.run(self.known_ips_reached_by());
        } else if let Some((check_duration, status_updates)) = self.hosts_up_checker.try_finish() {
            self.update_last_known_status(check_duration, status_updates);
        }
    }

    fn poll_dns_sd_discoverer(&mut self, force_refresh: bool) {
        if !self.cfg.read().service_discovery_enabled() {
            return;
        }
        if self.dns_sd_discoverer.is_time_to_run() || force_refresh {
            log::info!("Running DNS-SD discovery");
            self.dns_sd_discoverer.run();
        } else if let Some((check_duration, service_discovery_result)) =
            self.dns_sd_discoverer.try_finish()
        {
            self.update_service_instances(check_duration, service_discovery_result);
        }
    }

    fn run(&mut self) {
        while !self.stop_flag.load(Ordering::Relaxed) {
            let should_refresh = self.refresh_listener.do_refresh();
            if should_refresh {
                self.db.clear();
                self.update_msgs = vec![CollectorUpdate::Refresh];
            }
            self.poll_host_checker(should_refresh);
            self.poll_dns_sd_discoverer(should_refresh);

            while let Ok(ip_info) = self.rx_info.try_recv() {
                self.insert_or_update(ip_info);
            }

            // Send all modified ip info
            for msg in self.update_msgs.drain(..) {
                if let Err(e) = self.tx_info.send(msg) {
                    if self.stop_flag.load(Ordering::Relaxed) {
                        return;
                    } else {
                        panic!("Failed to send ip info: {e}");
                    }
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    fn known_ips_reached_by(&self) -> Vec<(IpAddr, ReachedBy)> {
        let mut v = Vec::with_capacity(self.db.len());

        for ipinfo in self.db.values() {
            let reached = ipinfo
                .reached_by()
                .expect("Unsound condition. IP info DB has host without information about how it was reached");
            let ip = ipinfo.ip();
            v.push((ip, reached));
        }
        v
    }

    fn update_last_known_status(
        &mut self,
        check_duration: Duration,
        status_updates: Vec<(IpAddr, (LastKnownStatus, Option<Duration>))>,
    ) {
        let mut online_count = 0;
        let mut offline_count = 0;
        for (ip, (status, rtt)) in status_updates {
            match status {
                LastKnownStatus::Online => online_count += 1,
                LastKnownStatus::Offline => offline_count += 1,
            }
            self.set_last_known_status(ip, (status, rtt));
        }
        log::info!(
            "✅ Known host check completed in {check_duration:.02?}: online={online_count}, offline={offline_count}"
        );
    }

    fn set_last_known_status(
        &mut self,
        ip: IpAddr,
        (status, rtt): (LastKnownStatus, Option<Duration>),
    ) {
        if let Some(ip_info) = self.db.get_mut(&ip) {
            ip_info.set_last_known_status((status, rtt));
            self.update_msgs
                .push(CollectorUpdate::Status((ip, (status, rtt))));
        }
    }

    fn update_service_instances(
        &mut self,
        check_duration: Duration,
        service_discovery_result: io::Result<Vec<ServiceInfo>>,
    ) {
        match service_discovery_result {
            Ok(service_instances) => {
                for service in service_instances {
                    let service_instance = ServiceInstance::new(
                        service.name,
                        service._type,
                        Some(service.host),
                        service.port,
                        service.txt,
                    );
                    let ip_info = IpInfo {
                        ip: service.ip,
                        reached_by: Some(ReachedBy::Mdns),
                        rtt: None,
                        names: vec![],
                        service_instances: Some(vec![service_instance]),
                        last_known_status: LastKnownStatus::Online,
                        seen_count: 1,
                        last_updated: Instant::now(),
                    };
                    self.insert_or_update(ip_info);
                }

                log::info!("✅ DNS-SD Discovery completed in {check_duration:.02?}: ");
            }
            Err(e) => {
                log::error!("DNS-SD Discovery failed: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use mds_util::refresh::Refresher;

    use super::*;
    use std::net::Ipv4Addr;
    use std::sync::mpsc;

    #[test]
    fn test_ip_info_collector_send_ip_info() {
        let cfg = SharedConfig::default();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let (tx_input, rx_input) = mpsc::channel();
        let (tx_output, rx_output) = mpsc::channel();
        let refreser = Refresher::new();
        let mut collector = IpInfoCollector::new(
            Arc::clone(&stop_flag),
            rx_input,
            tx_output,
            cfg,
            refreser.listen(),
        );

        // Test inserting new IP
        let mut ip_info_1 = IpInfo::from_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)));
        ip_info_1.add_name("test1.local".to_owned());

        tx_input.send(ip_info_1.clone()).unwrap();

        // Run collector
        let h_collector = thread::Builder::new()
            .name("test_ip_info_collector".into())
            .spawn(move || {
                collector.run();
                collector
            })
            .expect("failed spawning test thread");

        let received = rx_output.recv().unwrap();
        match received {
            CollectorUpdate::IpInfo(ip_info) => assert_eq!(ip_info, ip_info_1),
            _ => panic!("Unexpected message received"),
        }
        stop_flag.store(true, Ordering::SeqCst);
        let collector = h_collector.join().expect("failed joining collector handle");
        assert_eq!(collector.db.len(), 1);
        assert!(collector.update_msgs.is_empty());
    }

    #[test]
    fn test_ip_info_collector_db_empty() {
        let cfg = SharedConfig::default();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let (_tx_input, rx_input) = mpsc::channel();
        let (tx_output, _rx_output) = mpsc::channel();
        let refreser = Refresher::new();
        let mut collector = IpInfoCollector::new(
            Arc::clone(&stop_flag),
            rx_input,
            tx_output,
            cfg,
            refreser.listen(),
        );

        // Run collector
        let h_collector = std::thread::Builder::new()
            .name("test_ip_info_collector".into())
            .spawn(move || {
                collector.run();
                collector
            })
            .expect("failed spawning test thread");

        thread::sleep(Duration::from_millis(100));
        stop_flag.store(true, Ordering::SeqCst);
        let collector = h_collector.join().expect("Failed joining collector handle");
        assert!(collector.db.is_empty());
    }

    #[test]
    fn test_ip_info_collector_refresh() {
        let cfg = SharedConfig::default();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let (tx_input, rx_input) = mpsc::channel();
        let (tx_output, _rx_output) = mpsc::channel();
        let refresher = Refresher::new();
        let mut collector = IpInfoCollector::new(
            Arc::clone(&stop_flag),
            rx_input,
            tx_output,
            cfg,
            refresher.listen(),
        );

        // Send IP, expect that refresh clears it
        let mut ip_info_1 = IpInfo::from_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)));
        ip_info_1.add_name("test1.local".to_owned());
        tx_input.send(ip_info_1.clone()).unwrap();

        // Run collector
        let h_collector = std::thread::Builder::new()
            .name("test_ip_info_collector".into())
            .spawn(move || {
                collector.run();
                collector
            })
            .expect("failed spawning test thread");

        // There's a race condition here. If the stop flag is set after the refresher signal is checked in the run loop
        // then it will stop but it won't have reset. so we have to first wait for the refresher signal to have taken effect
        thread::sleep(Duration::from_millis(10)); // Allow receiving IP info
        refresher.signal();
        thread::sleep(Duration::from_millis(500)); // Allow refreshing
        stop_flag.store(true, Ordering::SeqCst);
        let collector = h_collector.join().expect("Failed joining collector handle");
        assert!(collector.db.is_empty());
    }
}
