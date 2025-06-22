use std::net::IpAddr;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use mds_config::AppConfig;
use mds_ipinfo::LastKnownStatus;
use mds_util::prelude::is_host_up;
use parking_lot::RwLock;

pub(super) struct HostsUpChecker {
    time_since_last_run: Instant,
    check_cooldown_secs: u16,
    handle: Option<JoinHandle<Vec<(IpAddr, LastKnownStatus)>>>,
    cfg: Arc<RwLock<AppConfig>>,
}

impl HostsUpChecker {
    pub(super) fn new(check_cooldown_secs: u16, cfg: Arc<RwLock<AppConfig>>) -> Self {
        Self {
            time_since_last_run: Instant::now(),
            check_cooldown_secs,
            handle: None,
            cfg,
        }
    }

    pub(super) fn reset(&mut self) {
        self.time_since_last_run = Instant::now();
        self.handle = None;
    }

    pub(super) fn run(&mut self, host_ips: Vec<IpAddr>) {
        self.time_since_last_run = Instant::now();
        let timeout_settings = self.cfg.read().timeout_settings();
        let h: JoinHandle<Vec<(IpAddr, LastKnownStatus)>> = thread::Builder::new()
            .name("host_up_checker".to_string())
            .spawn(move || {
                let mut status_updates = vec![];
                for ip in host_ips {
                    match ip {
                        IpAddr::V4(ipv4_addr) => {
                            let status = if is_host_up(ipv4_addr, None, timeout_settings) {
                                LastKnownStatus::Online
                            } else {
                                LastKnownStatus::Offline
                            };
                            status_updates.push((ip, status));
                        }
                        IpAddr::V6(_) => (),
                    }
                }
                status_updates
            })
            .expect("Failed to spawn host up checker thread");

        self.handle = Some(h);
    }

    pub(super) fn is_time_to_run(&self) -> bool {
        if !self.is_ready() {
            return false;
        }
        self.time_since_last_run.elapsed().as_secs() > self.check_cooldown_secs.into()
    }

    fn is_ready(&self) -> bool {
        self.handle.is_none()
    }

    pub(super) fn try_finish(&mut self) -> Option<(Duration, Vec<(IpAddr, LastKnownStatus)>)> {
        if let Some(h) = self.handle.take_if(|h| h.is_finished()) {
            let status_updates = h.join().expect("host up checker thread errored");
            let elapsed = self.time_since_last_run.elapsed();
            self.time_since_last_run = Instant::now();
            Some((elapsed, status_updates))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mds_ipinfo::LastKnownStatus;
    use mds_util::prelude::IP_TEST_NET_1_UNREACHABLE;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_checker_run_and_finish_with_mixed_ips() {
        let cfg = Arc::new(RwLock::new(AppConfig::default()));
        let mut checker = HostsUpChecker::new(0, cfg);
        let ips = vec![
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            IpAddr::V4(IP_TEST_NET_1_UNREACHABLE),
        ];

        checker.run(ips.clone());

        // Wait long enough for network timeouts
        thread::sleep(Duration::from_secs(2));

        let result = checker.try_finish();
        assert!(result.is_some(), "Expected the check to be finished.");

        let (_, status_updates) = result.unwrap();
        assert_eq!(status_updates.len(), 2);

        for (ip, status) in status_updates {
            if ip == ips[0] {
                assert!(
                    matches!(status, LastKnownStatus::Online),
                    "Expected 127.0.0.1 to be Online"
                );
            } else if ip == ips[1] {
                assert!(
                    matches!(status, LastKnownStatus::Offline),
                    "Expected {IP_TEST_NET_1_UNREACHABLE} to be Offline"
                );
            } else {
                panic!("Unexpected IP in results");
            }
        }
    }
}
