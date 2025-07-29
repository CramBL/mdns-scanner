use std::net::IpAddr;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use mds_config::shared_config::SharedConfig;
use mds_ipinfo::{IpForHost, LastKnownStatus};
use mds_util::host_up::{ReachedBy, up_by_tcp};
use mds_util::ping;

type HostCheckResult = (IpAddr, (LastKnownStatus, Option<Duration>));

pub(super) struct HostsUpChecker {
    time_since_last_run: Instant,
    check_cooldown_secs: u16,
    handle: Option<JoinHandle<Vec<HostCheckResult>>>,
    cfg: SharedConfig,
}

impl HostsUpChecker {
    pub(super) fn new(check_cooldown_secs: u16, cfg: SharedConfig) -> Self {
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

    pub(super) fn run(&mut self, host_ips: Vec<(IpForHost, ReachedBy)>) {
        self.time_since_last_run = Instant::now();
        let timeout_settings = self.cfg.read().timeout_settings();
        let h: JoinHandle<Vec<HostCheckResult>> = thread::Builder::new()
            .name("host_up_checker".to_string())
            .spawn(move || {
                let mut status_updates = vec![];
                for (ip, reached_by) in host_ips {
                    match ip {
                        IpForHost::V4andV6((ipv4, _)) | IpForHost::V4(ipv4) => {
                            let is_up = match reached_by {
                                ReachedBy::Port(port) => {
                                    up_by_tcp(ipv4, &[port], timeout_settings.tcp_port())
                                        .map(|(_port, rtt)| rtt)
                                }
                                ReachedBy::EchoReply => {
                                    ping::icmp_ping(ipv4, timeout_settings.ping())
                                }
                                ReachedBy::Mdns => None, // TODO: This will require a more sophisticated approach
                            };
                            let status = if is_up.is_some() {
                                (LastKnownStatus::Online, is_up)
                            } else {
                                (LastKnownStatus::Offline, is_up)
                            };
                            status_updates.push((IpAddr::V4(ipv4), status));
                        }
                        IpForHost::V6(_) => (),
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

    pub(super) fn try_finish(&mut self) -> Option<(Duration, Vec<HostCheckResult>)> {
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
    use std::net::Ipv4Addr;

    #[test]
    fn test_checker_run_and_finish_with_mixed_ips() {
        let cfg = SharedConfig::default();
        let mut checker = HostsUpChecker::new(0, cfg);
        let ips = vec![
            (
                IpForHost::V4(Ipv4Addr::new(127, 0, 0, 1)),
                ReachedBy::EchoReply,
            ),
            (
                IpForHost::V4(IP_TEST_NET_1_UNREACHABLE),
                ReachedBy::Port(80),
            ),
        ];

        checker.run(ips.clone());

        // Wait long enough for network timeouts
        thread::sleep(Duration::from_secs(2));

        let result = checker.try_finish();
        assert!(result.is_some(), "Expected the check to be finished.");

        let (_, status_updates) = result.unwrap();
        assert_eq!(status_updates.len(), 2);

        for (ip, status) in status_updates {
            let ip_for_host: IpForHost = ip.into();
            if ip_for_host == ips[0].0 {
                assert!(
                    matches!(status, (LastKnownStatus::Online, Some(_))),
                    "Expected 127.0.0.1 to be Online"
                );
            } else if ip_for_host == ips[1].0 {
                assert!(
                    matches!(status, (LastKnownStatus::Offline, None)),
                    "Expected {IP_TEST_NET_1_UNREACHABLE} to be Offline"
                );
            } else {
                panic!("Unexpected IP in results");
            }
        }
    }
}
