use std::{
    num::NonZero,
    thread,
    time::{Duration, Instant},
};

const FALLBACK_PARALLELISM: NonZero<usize> = NonZero::new(10).unwrap();
const MIN_PARALLELISM: usize = 1;

// Even if the target has 1 CPU, we will use a fair number of threads
const MIN_LOW_TIER_THREADS: usize = 32;
const MAX_IO_THREADS: usize = 8192;

enum PerformanceTier {
    High,
    Mid,
    Low,
}

impl PerformanceTier {
    pub fn from(available_parallelism: usize) -> Self {
        if available_parallelism >= 32 {
            PerformanceTier::High
        } else if available_parallelism >= 8 {
            PerformanceTier::Mid
        } else {
            PerformanceTier::Low
        }
    }

    pub fn thread_scale(&self) -> usize {
        match self {
            PerformanceTier::High | PerformanceTier::Mid => 32,
            PerformanceTier::Low => 16,
        }
    }

    pub fn passive_refresh_interval(&self) -> Duration {
        match self {
            PerformanceTier::High => Duration::from_millis(50),
            PerformanceTier::Mid => Duration::from_millis(100),
            PerformanceTier::Low => Duration::from_millis(300),
        }
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
        Self {
            parallelism,
            last_check: Instant::now(),
        }
    }

    pub fn max_threads(&mut self) -> usize {
        let calculated_threads = self.performance_tier().thread_scale() * self.parallelism;
        calculated_threads.clamp(MIN_LOW_TIER_THREADS, MAX_IO_THREADS)
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
