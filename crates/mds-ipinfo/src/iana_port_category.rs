use std::ops::RangeInclusive;

use strum::IntoEnumIterator as _;

/// The three IANA port ranges. Port 0 is excluded from [`IanaPortCategory::System`]:
/// it cannot be connected to.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    strum::Display,
    strum::EnumIter,
    strum::EnumCount,
)]
pub enum IanaPortCategory {
    #[strum(to_string = "System ports")]
    System,
    #[strum(to_string = "User ports")]
    User,
    #[strum(to_string = "Dynamic ports")]
    Dynamic,
}

impl IanaPortCategory {
    const SYSTEM_PORTS: RangeInclusive<u16> = 1..=1023;
    const USER_PORTS: RangeInclusive<u16> = 1024..=49151;
    const DYNAMIC_PORTS: RangeInclusive<u16> = 49152..=65535;

    pub fn of_port(port: u16) -> Option<Self> {
        Self::iter().find(|category| category.range().contains(&port))
    }

    pub fn categories_covering(ports: &[u16]) -> Vec<Self> {
        Self::iter()
            .filter(|category| {
                ports
                    .iter()
                    .any(|port| Self::of_port(*port) == Some(*category))
            })
            .collect()
    }

    /// Categories with no members are skipped. Each bucket keeps the input order of `ports`.
    pub fn group_ports_by_category(ports: &[u16]) -> Vec<(Self, Vec<u16>)> {
        debug_assert!(ports.is_sorted(), "ports must be sorted: {ports:?}");
        Self::iter()
            .filter_map(|category| {
                let members: Vec<u16> = ports
                    .iter()
                    .copied()
                    .filter(|port| Self::of_port(*port) == Some(category))
                    .collect();
                (!members.is_empty()).then_some((category, members))
            })
            .collect()
    }

    pub fn range(self) -> RangeInclusive<u16> {
        match self {
            Self::System => Self::SYSTEM_PORTS,
            Self::User => Self::USER_PORTS,
            Self::Dynamic => Self::DYNAMIC_PORTS,
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(0, None)]
    #[case(1, Some(IanaPortCategory::System))]
    #[case(1023, Some(IanaPortCategory::System))]
    #[case(1024, Some(IanaPortCategory::User))]
    #[case(49151, Some(IanaPortCategory::User))]
    #[case(49152, Some(IanaPortCategory::Dynamic))]
    #[case(65535, Some(IanaPortCategory::Dynamic))]
    fn of_port_classifies_boundaries(
        #[case] port: u16,
        #[case] expected: Option<IanaPortCategory>,
    ) {
        assert_eq!(IanaPortCategory::of_port(port), expected);
    }

    #[test]
    fn categories_covering_is_sorted_and_deduplicated() {
        let covered = IanaPortCategory::categories_covering(&[8009, 22, 80, 51413, 443]);
        assert_eq!(
            covered,
            vec![
                IanaPortCategory::System,
                IanaPortCategory::User,
                IanaPortCategory::Dynamic
            ]
        );
    }

    #[test]
    fn group_ports_by_category_buckets_ports_in_input_order() {
        let grouped = IanaPortCategory::group_ports_by_category(&[22, 80, 443, 8009, 51413]);
        assert_eq!(
            grouped,
            vec![
                (IanaPortCategory::System, vec![22, 80, 443]),
                (IanaPortCategory::User, vec![8009]),
                (IanaPortCategory::Dynamic, vec![51413]),
            ]
        );
    }
}
