use std::{
    thread,
    time::{Duration, Instant},
};

use mds_log::prelude::*;

use mds_dns_sd::prelude::*;

pub(super) struct DnsSdDiscoverer {
    time_since_last_run: Instant,
    check_cooldown_secs: u16,
    log: Logger,
    handle: Option<thread::JoinHandle<anyhow::Result<Vec<ServiceInfo>>>>,
}

impl DnsSdDiscoverer {
    pub(super) fn new(
        log: Logger,
        check_cooldown_secs: u16,
        service_discovery_enabled: bool,
    ) -> Self {
        // We spawn it immediately on creation
        let handle = if service_discovery_enabled {
            Self::spawn_discoverer(&log)
        } else {
            None
        };
        Self {
            time_since_last_run: Instant::now(),
            check_cooldown_secs,
            log,
            handle,
        }
    }

    fn spawn_discoverer(
        log: &Logger,
    ) -> Option<thread::JoinHandle<anyhow::Result<Vec<ServiceInfo>>>> {
        let h = match spawn_dns_sd_discoverer(log.clone()) {
            Ok(h) => h,
            Err(e) => {
                log.error(format!("Failed spawning DNS-SD discoverer: {e}"));
                return None;
            }
        };
        Some(h)
    }

    pub(super) fn run(&mut self) {
        if self.handle.is_some() {
            return;
        }
        self.time_since_last_run = Instant::now();

        self.handle = Self::spawn_discoverer(&self.log);
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

    pub(super) fn try_finish(&mut self) -> Option<(Duration, anyhow::Result<Vec<ServiceInfo>>)> {
        if let Some(h) = self.handle.take_if(|h| h.is_finished()) {
            let service_discovery_result = h.join().expect("DNS-SD discoverer thread errored");
            let elapsed = self.time_since_last_run.elapsed();
            self.time_since_last_run = Instant::now();

            Some((elapsed, service_discovery_result))
        } else {
            None
        }
    }
}
