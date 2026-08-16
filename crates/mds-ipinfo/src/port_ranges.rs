use std::fmt::{self, Display};
use std::ops::RangeInclusive;

/// The scanned ports collapsed into contiguous runs, e.g. `1-1023, 49152-65535`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PortRanges(Vec<RangeInclusive<u16>>);

impl PortRanges {
    /// Merges a sorted, deduplicated port list into its contiguous runs.
    pub fn merged_from(ports: &[u16]) -> Self {
        debug_assert!(
            ports.windows(2).all(|pair| pair[0] < pair[1]),
            "ports must be sorted and deduplicated: {ports:?}"
        );
        let mut runs: Vec<RangeInclusive<u16>> = Vec::new();
        for &port in ports {
            match runs.last_mut() {
                Some(run) if run.end().checked_add(1) == Some(port) => {
                    *run = *run.start()..=port;
                }
                _ => runs.push(port..=port),
            }
        }
        Self(runs)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn port_count(&self) -> usize {
        self.0
            .iter()
            .map(|run| usize::from(*run.end() - *run.start()) + 1)
            .sum()
    }
}

impl Display for PortRanges {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, run) in self.0.iter().enumerate() {
            if index > 0 {
                write!(f, ", ")?;
            }
            if run.start() == run.end() {
                write!(f, "{}", run.start())?;
            } else {
                write!(f, "{start}-{end}", start = run.start(), end = run.end())?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merged_from_collapses_contiguous_ports_into_one_run() {
        let ranges = PortRanges::merged_from(&[1, 2, 3, 4, 5]);
        assert_eq!(ranges.to_string(), "1-5");
        assert_eq!(ranges.port_count(), 5);
    }

    #[test]
    fn merged_from_keeps_gaps_as_separate_runs() {
        let ranges = PortRanges::merged_from(&[22, 80, 81, 82, 443]);
        assert_eq!(ranges.to_string(), "22, 80-82, 443");
        assert_eq!(ranges.port_count(), 5);
    }

    #[test]
    fn merged_from_adjacent_iana_ranges_collapse_to_one() {
        let ports: Vec<u16> = (1..=1023)
            .chain(1024..=49151)
            .chain(49152..=65535)
            .collect();
        let ranges = PortRanges::merged_from(&ports);
        assert_eq!(ranges.to_string(), "1-65535");
        assert_eq!(ranges.port_count(), 65535);
    }

    #[test]
    fn merged_from_non_adjacent_iana_ranges_stay_separate() {
        let ports: Vec<u16> = (1..=1023).chain(49152..=65535).collect();
        let ranges = PortRanges::merged_from(&ports);
        assert_eq!(ranges.to_string(), "1-1023, 49152-65535");
        assert_eq!(ranges.port_count(), 17407);
    }

    #[test]
    fn merged_from_no_ports_is_empty() {
        let ranges = PortRanges::merged_from(&[]);
        assert!(ranges.is_empty());
        assert_eq!(ranges.to_string(), "");
        assert_eq!(ranges.port_count(), 0);
    }

    #[test]
    fn merged_from_keeps_the_top_of_the_port_space_in_one_run() {
        let ranges = PortRanges::merged_from(&[65534, 65535]);
        assert_eq!(ranges.to_string(), "65534-65535");
        assert_eq!(ranges.port_count(), 2);
    }
}
