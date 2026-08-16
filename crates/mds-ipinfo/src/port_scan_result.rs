use std::num::NonZero;
use std::time::{Duration, Instant};

use crate::{IanaPortCategory, PortRanges};

/// Ports that answered but are not open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortScanOutcomeCounts {
    pub closed: u32,
    pub filtered: u32,
}

/// Result of the optional mDNS probe on 5353/udp, including which reply mode(s)
/// a responding host used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MdnsProbeOutcome {
    Silent,
    Unicast,
    Multicast,
    UnicastAndMulticast,
    /// The target is IPv6-only, which the IPv4 mDNS probe cannot reach.
    NotApplicable,
}

/// The parameters a port scan runs under, frozen when the scan starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortScanSettings {
    pub port_count: usize,
    /// For the header display.
    pub port_ranges: PortRanges,
    pub worker_count: NonZero<u16>,
    pub connect_timeout: Duration,
    /// Present when the scan included the unicast mDNS probe.
    pub mdns_probe_timeout: Option<Duration>,
}

impl PortScanSettings {
    /// The connect timeout times the number of scan rounds: the duration if every
    /// port is filtered, so every connect waits the full timeout.
    pub fn all_filtered_duration(&self) -> Duration {
        self.connect_timeout * (self.port_count as u32).div_ceil(self.worker_count.get().into())
    }
}

/// Outcome of the most recent port scan of one host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortScanResult {
    open_ports: Vec<u16>,
    scanned: u32,
    counts: PortScanOutcomeCounts,
    mdns: Option<MdnsProbeOutcome>,
    duration: Duration,
    cancelled: bool,
    settings: PortScanSettings,
    scanned_categories: Vec<IanaPortCategory>,
    completed_at: Instant,
}

impl PortScanResult {
    /// `open_ports` is displayed in the given order, so it must be sorted ascending.
    /// `mdns` is `None` when the scan did not include the mDNS probe.
    /// `scanned_categories` are the [`IanaPortCategory`]s the scanned port set covered,
    /// sorted and deduplicated.
    pub fn new(
        open_ports: Vec<u16>,
        counts: PortScanOutcomeCounts,
        mdns: Option<MdnsProbeOutcome>,
        duration: Duration,
        cancelled: bool,
        settings: PortScanSettings,
        scanned_categories: Vec<IanaPortCategory>,
    ) -> Self {
        debug_assert!(
            open_ports.is_sorted(),
            "unsorted open ports: {open_ports:?}"
        );
        debug_assert!(
            scanned_categories.is_sorted(),
            "unsorted scanned categories: {scanned_categories:?}"
        );
        Self {
            scanned: open_ports.len() as u32 + counts.closed + counts.filtered,
            open_ports,
            counts,
            mdns,
            duration,
            cancelled,
            settings,
            scanned_categories,
            completed_at: Instant::now(),
        }
    }

    pub fn open_ports(&self) -> &[u16] {
        &self.open_ports
    }

    pub fn open_count(&self) -> u32 {
        self.open_ports.len() as u32
    }

    pub fn scanned(&self) -> u32 {
        self.scanned
    }

    pub fn closed(&self) -> u32 {
        self.counts.closed
    }

    pub fn filtered(&self) -> u32 {
        self.counts.filtered
    }

    pub fn mdns(&self) -> Option<MdnsProbeOutcome> {
        self.mdns
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }

    pub fn was_cancelled(&self) -> bool {
        self.cancelled
    }

    pub fn settings(&self) -> &PortScanSettings {
        &self.settings
    }

    pub fn scanned_categories(&self) -> &[IanaPortCategory] {
        &self.scanned_categories
    }

    pub fn completed_at(&self) -> Instant {
        self.completed_at
    }

    pub fn age(&self) -> Duration {
        self.completed_at.elapsed()
    }

    pub fn open_ports_comma_separated(&self) -> String {
        self.open_ports
            .iter()
            .map(u16::to_string)
            .collect::<Vec<String>>()
            .join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COUNTS: PortScanOutcomeCounts = PortScanOutcomeCounts {
        closed: 1000,
        filtered: 22,
    };

    fn settings() -> PortScanSettings {
        PortScanSettings {
            port_count: 1024,
            port_ranges: PortRanges::merged_from(&(1..=1024).collect::<Vec<u16>>()),
            worker_count: NonZero::new(64).unwrap(),
            connect_timeout: Duration::from_millis(100),
            mdns_probe_timeout: None,
        }
    }

    fn result(
        open_ports: Vec<u16>,
        mdns: Option<MdnsProbeOutcome>,
        cancelled: bool,
    ) -> PortScanResult {
        PortScanResult::new(
            open_ports,
            COUNTS,
            mdns,
            Duration::from_secs(1),
            cancelled,
            settings(),
            vec![IanaPortCategory::System],
        )
    }

    #[test]
    fn scanned_count_is_the_sum_of_all_outcomes() {
        let result = result(vec![22, 80], None, false);
        assert_eq!(result.scanned(), 1024);
        assert_eq!(result.open_count(), 2);
    }

    #[test]
    fn new_returns_the_given_mdns_probe_outcome() {
        let result = result(vec![], Some(MdnsProbeOutcome::UnicastAndMulticast), false);
        assert_eq!(result.mdns(), Some(MdnsProbeOutcome::UnicastAndMulticast));
    }

    #[test]
    fn all_filtered_duration_is_the_timeout_times_the_number_of_rounds() {
        assert_eq!(
            settings().all_filtered_duration(),
            Duration::from_millis(1600)
        );
    }

    #[test]
    fn open_ports_comma_separated_joins_in_order() {
        let result = result(vec![53, 443, 8009], None, false);
        assert_eq!(result.open_ports_comma_separated(), "53, 443, 8009");
    }

    #[test]
    fn open_ports_comma_separated_of_a_result_without_open_ports_is_empty() {
        let result = result(vec![], None, true);
        assert_eq!(result.open_ports_comma_separated(), "");
    }
}
