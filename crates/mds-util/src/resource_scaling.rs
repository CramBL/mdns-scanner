use std::{
    num::NonZero,
    thread,
    time::{Duration, Instant},
};

use mds_config::scan::io_threads::{MAX_IO_THREADS, MIN_LOW_TIER_THREADS};

const FALLBACK_PARALLELISM: NonZero<usize> = NonZero::new(10).unwrap();
const MIN_PARALLELISM: usize = 1;

// Default NOFILE on linux
const DEFAULT_MAX_FD: u64 = 1024;

enum PerformanceTier {
    High(usize),
    Mid(usize),
    Low(usize),
}

impl PerformanceTier {
    pub fn from(available_parallelism: usize) -> Self {
        if available_parallelism >= 32 {
            PerformanceTier::High(available_parallelism)
        } else if available_parallelism >= 8 {
            PerformanceTier::Mid(available_parallelism)
        } else {
            PerformanceTier::Low(available_parallelism)
        }
    }

    pub fn thread_scale(&self) -> usize {
        match self {
            PerformanceTier::High(_) | PerformanceTier::Mid(_) => 32,
            PerformanceTier::Low(_) => 16,
        }
    }

    pub fn passive_refresh_interval(&self) -> Duration {
        match self {
            PerformanceTier::High(_) => Duration::from_millis(50),
            PerformanceTier::Mid(_) => Duration::from_millis(100),
            PerformanceTier::Low(_) => Duration::from_millis(300),
        }
    }

    pub fn max_threads(&self) -> usize {
        match self {
            PerformanceTier::High(parallelism)
            | PerformanceTier::Mid(parallelism)
            | PerformanceTier::Low(parallelism) => self.thread_scale() * parallelism,
        }
    }

    pub fn max_file_descriptors(&self) -> u64 {
        // 2 for each scanner thread, one TCP socket and one ICMP/raw socket
        // we choose 3 for the extras for sleeping sockets, host up checking, DNS-SD discovery and misc.
        (self.max_threads() * 3) as u64
    }
}

fn get_available_parallelism() -> usize {
    let available_parallelism = thread::available_parallelism()
        .unwrap_or(FALLBACK_PARALLELISM)
        .get();
    available_parallelism.max(MIN_PARALLELISM)
}

#[derive(Clone, Copy)]
pub struct HostResources {
    fd_limit: u64,
    parallelism: usize,
    last_check: Instant,
}

impl Default for HostResources {
    fn default() -> Self {
        Self::new()
    }
}

impl HostResources {
    pub fn new() -> Self {
        let parallelism = get_available_parallelism();
        let perf_tier = PerformanceTier::from(parallelism);
        let max_fd = perf_tier.max_file_descriptors();
        let fd_limit = rlimit::increase_nofile_limit(max_fd).unwrap_or(DEFAULT_MAX_FD);

        Self {
            fd_limit,
            parallelism,
            last_check: Instant::now(),
        }
    }

    pub fn max_file_descriptors(&mut self) -> u64 {
        let curr_max_fd = self.performance_tier().max_file_descriptors();
        if self.fd_limit < curr_max_fd {
            self.fd_limit = rlimit::increase_nofile_limit(curr_max_fd).unwrap_or(DEFAULT_MAX_FD);
        }
        curr_max_fd
    }

    pub fn max_threads(&mut self) -> NonZero<u16> {
        let calculated_threads = self.performance_tier().max_threads();
        let max_threads = calculated_threads.clamp(MIN_LOW_TIER_THREADS, MAX_IO_THREADS);
        let scaled_max_fd = (self.max_file_descriptors() / 3) as usize;
        NonZero::new(max_threads.min(scaled_max_fd) as u16).unwrap()
    }

    pub fn passive_refresh_interval(&mut self) -> Duration {
        self.performance_tier().passive_refresh_interval()
    }

    fn maybe_refresh(&mut self) {
        if self.last_check.elapsed() > Duration::from_secs(10) {
            self.parallelism = get_available_parallelism();
            self.last_check = Instant::now();
        }
    }

    fn performance_tier(&mut self) -> PerformanceTier {
        self.maybe_refresh();
        PerformanceTier::from(self.parallelism)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_resource_management_workflow() {
        let mut resources = HostResources::new();

        println!(
            "System detected {} CPU cores(-esque)",
            resources.parallelism
        );

        let max_threads = resources.max_threads();
        let max_fds = resources.max_file_descriptors();
        let refresh_interval = resources.passive_refresh_interval();

        println!(
            "Configured for {max_threads} threads, {max_fds} FDs, {}ms refresh",
            refresh_interval.as_millis()
        );

        // Verify the configuration makes sense
        let max_threads = max_threads.get();
        assert!(max_threads > 0);
        assert!(max_fds >= max_threads as u64 * 3); // At least 3 FDs per thread
        assert!(refresh_interval.as_millis() >= 50);
        assert!(refresh_interval.as_millis() <= 300);
    }
}
