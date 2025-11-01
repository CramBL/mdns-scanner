use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RttStats {
    first: Duration,
    latest: Duration,
    pub avg: Duration,
    pub min: Duration,
    pub max: Duration,
    count: u64,
}

impl RttStats {
    pub(crate) fn new(first: Duration) -> Self {
        Self {
            first,
            latest: first,
            avg: first,
            min: first,
            max: first,
            count: 1,
        }
    }

    #[inline]
    pub fn on_discover(&self) -> Duration {
        self.first
    }

    #[inline]
    pub fn latest(&self) -> Duration {
        self.latest
    }

    pub(crate) fn update(&mut self, new_rtt: Duration) {
        self.count += 1;
        self.latest = new_rtt;

        let avg_secs = self.avg.as_secs_f32();
        let new_secs = new_rtt.as_secs_f32();

        let updated_avg_secs = avg_secs + (new_secs - avg_secs) / self.count as f32;

        self.avg = Duration::from_secs_f32(updated_avg_secs);
        self.min = new_rtt.min(self.min);
        self.max = new_rtt.max(self.max);
    }

    /// Merges another `RttStats` into this one.
    ///
    /// Works with the assumption that `self` is an "older" instance
    /// and rioritizes `self` over `other` for fields with no meaningful merge strategy
    pub(crate) fn merge(&mut self, other: Self) {
        let Self {
            first: _,
            latest,
            avg,
            min,
            max,
            count,
        } = other;

        // since `self` is oldest, we overwrite with latest from `other`
        self.latest = latest;
        self.min = self.min.min(min);
        self.max = self.max.max(max);

        let total_count = self.count + count;
        let self_total_secs = self.avg.as_secs_f32() * self.count as f32;
        let other_total_secs = avg.as_secs_f32() * count as f32;
        let combined_total_secs = self_total_secs + other_total_secs;
        let updated_avg_secs = combined_total_secs / total_count as f32;
        self.avg = Duration::from_secs_f32(updated_avg_secs);
        self.count = total_count;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rtt_stats_merge_correctly_calculates_weighted_average() {
        const EPSILON: Duration = Duration::from_nanos(10);

        // Stats A: 2 samples, Avg = 10ms (Total RTT sum = 20ms)
        let rtt_a_initial = Duration::from_millis(10);
        let mut stats_a = RttStats::new(rtt_a_initial);
        stats_a.update(rtt_a_initial);

        // Stats B: 1 sample, Avg = 100ms (Total RTT sum = 100ms)
        let stats_b = RttStats::new(Duration::from_millis(100));
        stats_a.merge(stats_b);

        let expected_avg = Duration::from_millis(40);

        let diff = stats_a.avg.abs_diff(expected_avg);

        assert!(
            diff < EPSILON,
            "Averages diverged by {diff:?}. Calculated: {:?}, Expected: {expected_avg:?}",
            stats_a.avg,
        );

        assert_eq!(stats_a.count, 3);
        assert_eq!(stats_a.min, Duration::from_millis(10));
        assert_eq!(stats_a.max, Duration::from_millis(100));
    }
}
