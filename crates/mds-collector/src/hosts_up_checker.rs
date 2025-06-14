use std::net::IpAddr;
use std::thread;
use std::time::{Duration, Instant};

use mds_ipinfo::LastKnownStatus;
use mds_util::prelude::is_host_up;

pub(super) struct HostsUpChecker {
    time_since_last_run: Instant,
    check_cooldown_secs: u16,
    handle: Option<thread::JoinHandle<Vec<(IpAddr, LastKnownStatus)>>>,
}

impl HostsUpChecker {
    pub(super) fn new(check_cooldown_secs: u16) -> Self {
        Self {
            time_since_last_run: Instant::now(),
            check_cooldown_secs,
            handle: None,
        }
    }

    pub(super) fn run(&mut self, host_ips: Vec<IpAddr>) {
        if self.handle.is_some() {
            return;
        }
        self.time_since_last_run = Instant::now();
        let h: thread::JoinHandle<Vec<(IpAddr, LastKnownStatus)>> = std::thread::Builder::new()
            .name("host_up_checker".to_string())
            .spawn(move || {
                let mut status_updates = vec![];
                for ip in host_ips {
                    match ip {
                        IpAddr::V4(ipv4_addr) => {
                            let status = if is_host_up(ipv4_addr, None) {
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
