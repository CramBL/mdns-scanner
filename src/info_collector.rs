//! Ip info collector receives [IpInfo] and sends new or modified [IpInfo] to the TUI
use dns_sd_discoverer::DnsSdDiscoverer;
use hosts_up_checker::HostsUpChecker;

use crate::dns_sd::ServiceInfo;
use crate::ip_info::{IpInfo, LastKnownStatus};
use crate::log::logger::Logger;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

mod dns_sd_discoverer;
mod hosts_up_checker;

pub fn spawn_collector(
    stop_flag: Arc<AtomicBool>,
    rx_from_scanners: Receiver<IpInfo>,
    tx_to_table_pane: Sender<CollectorUpdate>,
    logger: Logger,
) {
    let mut collector = IpInfoCollector::new(stop_flag, rx_from_scanners, tx_to_table_pane, logger);
    std::thread::spawn(move || {
        collector.run();
    });
}

#[derive(Debug)]
pub enum CollectorUpdate {
    IpInfo(IpInfo),
    PacketSeen(IpAddr),
    Status((IpAddr, LastKnownStatus)),
}

struct IpInfoCollector {
    db: HashMap<IpAddr, IpInfo>,
    logger: Logger,
    rx_info: Receiver<IpInfo>,
    tx_info: Sender<CollectorUpdate>,
    stop_flag: Arc<AtomicBool>,
    update_msgs: Vec<CollectorUpdate>,
    hosts_up_checker: HostsUpChecker,
    dns_sd_discoverer: DnsSdDiscoverer,
}

impl IpInfoCollector {
    // How often to check for known hosts being up (time since last check)
    const HOST_UP_CHECK_INTERVAL_SECS: u8 = 16;
    const DNS_SD_DISCOVERY_INTERVAL_SECS: u8 = 30;

    fn new(
        stop_flag: Arc<AtomicBool>,
        rx_info: Receiver<IpInfo>,
        tx_info: Sender<CollectorUpdate>,
        logger: Logger,
    ) -> Self {
        Self {
            db: HashMap::new(),
            logger: logger.clone(),
            rx_info,
            tx_info,
            stop_flag,
            update_msgs: vec![],
            hosts_up_checker: HostsUpChecker::new(Self::HOST_UP_CHECK_INTERVAL_SECS.into()),
            dns_sd_discoverer: DnsSdDiscoverer::new(
                logger,
                Self::DNS_SD_DISCOVERY_INTERVAL_SECS.into(),
            ),
        }
    }

    fn insert(&mut self, ip_info: IpInfo) {
        self.db.insert(ip_info.ip, ip_info);
    }

    fn insert_or_update(&mut self, mut new_ip_info: IpInfo) {
        let ip = new_ip_info.ip;
        if let Some(ip_info) = self.db.get_mut(&ip) {
            if *ip_info != new_ip_info {
                let mut item_modified = false;
                for n in new_ip_info.names {
                    if !ip_info.contains(&n) {
                        ip_info.names.push(n);
                        ip_info.names.sort();
                        item_modified = true;
                    }
                }
                for mut service in new_ip_info.service_instances.into_iter().flatten() {
                    // Remove the service's hostname string if the service is available under
                    // the same hostname as the host machine, otherwise it is advertising under an
                    // independent mDNS hostname
                    let _ = service.hostname.take_if(|h| ip_info.names().contains(h));
                    if ip_info.update_with_service_instance(service) {
                        item_modified = true;
                    }
                }

                ip_info.seen_count += 1;
                if item_modified {
                    self.update_msgs
                        .push(CollectorUpdate::IpInfo(ip_info.clone()));
                } else {
                    self.update_msgs.push(CollectorUpdate::PacketSeen(ip));
                }
            }
        } else {
            new_ip_info.names.dedup();
            new_ip_info.names.sort();
            self.insert(new_ip_info.clone());
            self.update_msgs.push(CollectorUpdate::IpInfo(new_ip_info));
        }
    }

    fn poll_host_checker(&mut self) {
        if self.hosts_up_checker.is_time_to_run() {
            self.logger.info("Running status check for known hosts");
            self.hosts_up_checker.run(self.known_ips());
        } else if let Some((check_duration, status_updates)) = self.hosts_up_checker.try_finish() {
            self.update_last_known_status(check_duration, status_updates);
        }
    }

    fn poll_dns_sd_discoverer(&mut self) {
        if self.dns_sd_discoverer.is_time_to_run() {
            self.logger.info("Running DNS-SD discovery");
            self.dns_sd_discoverer.run();
        } else if let Some((check_duration, service_discovery_result)) =
            self.dns_sd_discoverer.try_finish()
        {
            self.update_service_instances(check_duration, service_discovery_result);
        }
    }

    fn run(&mut self) {
        loop {
            self.poll_host_checker();
            self.poll_dns_sd_discoverer();

            while let Ok(ip_info) = self.rx_info.try_recv() {
                self.insert_or_update(ip_info);
            }

            // Send all modified ip info
            for msg in self.update_msgs.drain(..) {
                if let Err(e) = self.tx_info.send(msg) {
                    if self.stop_flag.load(Ordering::SeqCst) {
                        return;
                    } else {
                        panic!("Failed to send ip info: {}", e);
                    }
                }
            }
            if self.stop_flag.load(Ordering::Relaxed) {
                return;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    fn known_ips(&self) -> Vec<IpAddr> {
        self.db.keys().map(|k| k.to_owned()).collect()
    }

    fn update_last_known_status(
        &mut self,
        check_duration: Duration,
        status_updates: Vec<(IpAddr, LastKnownStatus)>,
    ) {
        let mut online_count = 0;
        let mut offline_count = 0;
        for (ip, status) in status_updates {
            match status {
                LastKnownStatus::Online => online_count += 1,
                LastKnownStatus::Offline => offline_count += 1,
            }
            self.set_last_known_status(ip, status);
        }
        self.logger.info(format!(
                    "✅ Known host check completed in {check_duration:.02?}: online={online_count}, offline={offline_count}"
                ));
    }

    fn set_last_known_status(&mut self, ip: IpAddr, status: LastKnownStatus) {
        if let Some(ip_info) = self.db.get_mut(&ip) {
            if ip_info.last_known_status != status {
                ip_info.last_known_status = status;
                self.update_msgs.push(CollectorUpdate::Status((ip, status)));
            }
        }
    }

    fn update_service_instances(
        &mut self,
        check_duration: Duration,
        service_discovery_result: anyhow::Result<Vec<ServiceInfo>>,
    ) {
        match service_discovery_result {
            Ok(service_instances) => {
                for service in service_instances {
                    self.insert_or_update(service.into());
                }

                self.logger.info(format!(
                    "✅ DNS-SD Discovery completed in {check_duration:.02?}: "
                ));
            }
            Err(e) => {
                self.logger.error(format!("DNS-SD Discovery failed: {e}"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use std::sync::mpsc;

    #[test]
    fn test_ip_info_collector_send_ip_info() {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let (tx_input, rx_input) = mpsc::channel();
        let (tx_output, rx_output) = mpsc::channel();
        let (tx_logs, _rx_logs) = mpsc::channel();
        let logger = Logger::new(tx_logs, crate::log::LogLevel::default());

        let mut collector = IpInfoCollector::new(stop_flag, rx_input, tx_output, logger);

        // Test inserting new IP
        let mut ip_info_1 = IpInfo::from_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)));
        ip_info_1.names.push("test1.local".to_string());

        tx_input.send(ip_info_1.clone()).unwrap();

        // Run collector
        std::thread::spawn(move || {
            collector.run();
        });

        let received = rx_output.recv().unwrap();
        match received {
            CollectorUpdate::IpInfo(ip_info) => assert_eq!(ip_info, ip_info_1),
            _ => panic!("Unexpected message received"),
        }
    }
}
