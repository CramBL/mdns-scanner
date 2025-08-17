use std::{
    cmp::Ordering,
    fmt,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use color_eyre::eyre::bail;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IpForHost {
    V4(Ipv4Addr),
    V6(Ipv6Addr),
    V4andV6((Ipv4Addr, Ipv6Addr)),
}

impl IpForHost {
    /// Checks if this instance contains any of the same IPs as another.
    /// E.g., V4(A) shares an IP with V4andV6((A, B)).
    pub fn shares_ip_with(&self, other: &Self) -> bool {
        let (self_v4, self_v6) = self.get_ips();
        let (other_v4, other_v6) = other.get_ips();

        // Return true if the IPv4 addresses match (and exist) OR if the IPv6 addresses match.
        (self_v4.is_some() && self_v4 == other_v4) || (self_v6.is_some() && self_v6 == other_v6)
    }

    /// Helper to deconstruct the enum into its possible IP addresses.
    fn get_ips(&self) -> (Option<Ipv4Addr>, Option<Ipv6Addr>) {
        match *self {
            Self::V4(v4) => (Some(v4), None),
            Self::V6(v6) => (None, Some(v6)),
            Self::V4andV6((v4, v6)) => (Some(v4), Some(v6)),
        }
    }

    /// Merges two [`IpForHost`] instances.
    ///
    /// The logic prioritizes creating or preserving the `V4andV6` variant
    /// as it represents the most complete state.
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            // If one is V4 and the other is V6, combine them into a single V4andV6.
            (Self::V4(v4), Self::V6(v6)) | (Self::V6(v6), Self::V4(v4)) => Self::V4andV6((v4, v6)),

            // If `other` is already a complete pair, it's the desired state.
            (Self::V4(_), pair @ Self::V4andV6(_))
            | (Self::V6(_), pair @ Self::V4andV6(_))
            // If `self` is already a complete pair, it's the desired state.
            | (pair @ Self::V4andV6(_), _) => pair,


            // For all other combinations (e.g., V4+V4, V6+V6), the merge does not
            // produce a more complete type. We return `self` by convention.
            _ => self,
        }
    }

    pub fn max_unicode_width(&self) -> u16 {
        match self {
            IpForHost::V4(v4) => ipv4_width(*v4),
            IpForHost::V6(v6) => v6.to_string().width() as u16,
            IpForHost::V4andV6((v4, v6)) => ipv4_width(*v4).max(v6.to_string().width() as u16),
        }
    }
}

impl PartialOrd for IpForHost {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IpForHost {
    fn cmp(&self, other: &Self) -> Ordering {
        let (self_v4, self_v6) = match self {
            IpForHost::V4(v4) => (Some(*v4), None),
            IpForHost::V6(v6) => (None, Some(*v6)),
            IpForHost::V4andV6((v4, v6)) => (Some(*v4), Some(*v6)),
        };
        let (other_v4, other_v6) = match other {
            IpForHost::V4(v4) => (Some(*v4), None),
            IpForHost::V6(v6) => (None, Some(*v6)),
            IpForHost::V4andV6((v4, v6)) => (Some(*v4), Some(*v6)),
        };

        // Primary sort by Ipv4
        match (self_v4, other_v4) {
            (Some(s_v4), Some(o_v4)) => {
                let v4_cmp = s_v4.cmp(&o_v4);
                if v4_cmp != Ordering::Equal {
                    return v4_cmp;
                }
            }
            (Some(_), None) => return Ordering::Greater, // V4 comes after no V4
            (None, Some(_)) => return Ordering::Less,    // No V4 comes before V4
            (None, None) => { /* fall through to V6 comparison if both are None for V4 */ }
        }

        // Secondary sort by Ipv6 (None comes first)
        match (self_v6, other_v6) {
            (Some(s_v6), Some(o_v6)) => s_v6.cmp(&o_v6),
            (Some(_), None) => Ordering::Greater, // V6 comes after no V6
            (None, Some(_)) => Ordering::Less,    // No V6 comes before V6
            (None, None) => Ordering::Equal,      // Both None for both V4 and V6
        }
    }
}

impl From<IpAddr> for IpForHost {
    fn from(ip: IpAddr) -> Self {
        match ip {
            IpAddr::V4(ipv4) => Self::V4(ipv4),
            IpAddr::V6(ipv6) => Self::V6(ipv6),
        }
    }
}

impl TryFrom<(Option<Ipv4Addr>, Option<Ipv6Addr>)> for IpForHost {
    type Error = color_eyre::eyre::ErrReport;

    fn try_from(
        (ipv4_opt, ipv6_opt): (Option<Ipv4Addr>, Option<Ipv6Addr>),
    ) -> Result<Self, Self::Error> {
        match (ipv4_opt, ipv6_opt) {
            (None, None) => {
                bail!("Need either Ipv4 or Ipv6")
            }
            (None, Some(ipv6)) => Ok(Self::V6(ipv6)),
            (Some(ipv4), None) => Ok(Self::V4(ipv4)),
            (Some(ipv4), Some(ipv6)) => Ok(Self::V4andV6((ipv4, ipv6))),
        }
    }
}

impl fmt::Display for IpForHost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::V4(ipv4) => write!(f, "{ipv4}"),
            Self::V6(ipv6) => write!(f, "{ipv6}"),
            Self::V4andV6((ipv4, ipv6)) => write!(f, "{ipv4}\n{ipv6}"),
        }
    }
}

/// Calculate the display width of an IPv4 address without string allocation
#[inline]
fn ipv4_width(ip: Ipv4Addr) -> u16 {
    let octets = ip.octets();
    let mut width = 3; // 3 dots

    for &octet in &octets {
        width += match octet {
            0..=9 => 1,
            10..=99 => 2,
            100..=255 => 3,
        };
    }

    width
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn shares_ip_with_v4_exact_match() {
        let ip1 = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip2 = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1));
        assert!(ip1.shares_ip_with(&ip2));
    }

    #[test]
    fn shares_ip_with_v4_no_match() {
        let ip1 = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip2 = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 2));
        assert!(!ip1.shares_ip_with(&ip2));
    }

    #[test]
    fn shares_ip_with_v6_exact_match() {
        let ip1 = IpForHost::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        let ip2 = IpForHost::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        assert!(ip1.shares_ip_with(&ip2));
    }

    #[test]
    fn shares_ip_with_v6_no_match() {
        let ip1 = IpForHost::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        let ip2 = IpForHost::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2));
        assert!(!ip1.shares_ip_with(&ip2));
    }

    #[test]
    fn shares_ip_with_v4_and_v4andv6_match() {
        let ip1 = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip2 = IpForHost::V4andV6((
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
        ));
        assert!(ip1.shares_ip_with(&ip2));
        assert!(ip2.shares_ip_with(&ip1));
    }

    #[test]
    fn shares_ip_with_v6_and_v4andv6_match() {
        let ip1 = IpForHost::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        let ip2 = IpForHost::V4andV6((
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
        ));
        assert!(ip1.shares_ip_with(&ip2));
        assert!(ip2.shares_ip_with(&ip1));
    }

    #[test]
    fn shares_ip_with_v4_and_v6_no_match() {
        let ip1 = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip2 = IpForHost::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        assert!(!ip1.shares_ip_with(&ip2));
    }

    #[test]
    fn shares_ip_with_v4andv6_exact_match() {
        let ip1 = IpForHost::V4andV6((
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
        ));
        let ip2 = IpForHost::V4andV6((
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
        ));
        assert!(ip1.shares_ip_with(&ip2));
    }

    #[test]
    fn shares_ip_with_v4andv6_partial_match_v4() {
        let ip1 = IpForHost::V4andV6((
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
        ));
        let ip2 = IpForHost::V4andV6((
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2),
        ));
        assert!(ip1.shares_ip_with(&ip2));
    }

    #[test]
    fn shares_ip_with_v4andv6_partial_match_v6() {
        let ip1 = IpForHost::V4andV6((
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
        ));
        let ip2 = IpForHost::V4andV6((
            Ipv4Addr::new(192, 168, 1, 2),
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
        ));
        assert!(ip1.shares_ip_with(&ip2));
    }

    #[test]
    fn shares_ip_with_v4andv6_no_match() {
        let ip1 = IpForHost::V4andV6((
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
        ));
        let ip2 = IpForHost::V4andV6((
            Ipv4Addr::new(192, 168, 1, 2),
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2),
        ));
        assert!(!ip1.shares_ip_with(&ip2));
    }

    #[test]
    fn merge_v4_and_v6() {
        let v4 = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1));
        let v6 = IpForHost::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        let expected = IpForHost::V4andV6((
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
        ));
        assert_eq!(v4.merge(v6), expected);
        assert_eq!(v6.merge(v4), expected);
    }

    #[test]
    fn merge_v4_with_v4andv6() {
        let v4 = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1));
        let v4andv6 = IpForHost::V4andV6((
            Ipv4Addr::new(10, 0, 0, 1),
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
        ));
        assert_eq!(v4.merge(v4andv6), v4andv6);
    }

    #[test]
    fn merge_v6_with_v4andv6() {
        let v6 = IpForHost::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        let v4andv6 = IpForHost::V4andV6((
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1),
        ));
        assert_eq!(v6.merge(v4andv6), v4andv6);
    }

    #[test]
    fn merge_v4andv6_with_any() {
        let v4andv6 = IpForHost::V4andV6((
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
        ));
        let v4 = IpForHost::V4(Ipv4Addr::new(10, 0, 0, 1));
        let v6 = IpForHost::V6(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1));
        let other_v4andv6 = IpForHost::V4andV6((
            Ipv4Addr::new(172, 16, 0, 1),
            Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 1),
        ));

        assert_eq!(v4andv6.merge(v4), v4andv6);
        assert_eq!(v4andv6.merge(v6), v4andv6);
        assert_eq!(v4andv6.merge(other_v4andv6), v4andv6);
    }

    #[test]
    fn merge_same_type_returns_self() {
        let v4_1 = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1));
        let v4_2 = IpForHost::V4(Ipv4Addr::new(10, 0, 0, 1));
        let v6_1 = IpForHost::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        let v6_2 = IpForHost::V6(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1));

        assert_eq!(v4_1.merge(v4_2), v4_1);
        assert_eq!(v6_1.merge(v6_2), v6_1);
    }

    #[test]
    fn ordering_v4_only() {
        let ip1 = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip2 = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 2));
        let ip3 = IpForHost::V4(Ipv4Addr::new(10, 0, 0, 1));

        assert!(ip3 < ip1);
        assert!(ip1 < ip2);
        assert!(ip3 < ip2);
    }

    #[test]
    fn ordering_v6_only() {
        let ip1 = IpForHost::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        let ip2 = IpForHost::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2));
        let ip3 = IpForHost::V6(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1));

        assert!(ip1 < ip2);
        assert!(ip2 < ip3);
        assert!(ip1 < ip3);
    }

    #[test]
    fn ordering_v6_before_v4() {
        let v6 = IpForHost::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        let v4 = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1));

        assert!(v6 < v4);
    }

    #[test]
    fn ordering_v4andv6_primary_by_v4() {
        let ip1 = IpForHost::V4andV6((
            Ipv4Addr::new(10, 0, 0, 1),
            Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1),
        ));
        let ip2 = IpForHost::V4andV6((
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
        ));

        assert!(ip1 < ip2);
    }

    #[test]
    fn ordering_v4andv6_secondary_by_v6() {
        let ip1 = IpForHost::V4andV6((
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
        ));
        let ip2 = IpForHost::V4andV6((
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2),
        ));

        assert!(ip1 < ip2);
    }

    #[test]
    fn ordering_mixed_types() {
        let v6_only = IpForHost::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        let v4_only = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1));
        let v4andv6 = IpForHost::V4andV6((
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
        ));

        assert!(v6_only < v4_only);
        assert!(v4_only < v4andv6);
        assert!(v6_only < v4andv6);
    }

    #[test]
    fn from_ipaddr_v4() {
        let ipv4 = Ipv4Addr::new(192, 168, 1, 1);
        let ip_addr = IpAddr::V4(ipv4);
        let ip_for_host = IpForHost::from(ip_addr);
        assert_eq!(ip_for_host, IpForHost::V4(ipv4));
    }

    #[test]
    fn from_ipaddr_v6() {
        let ipv6 = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        let ip_addr = IpAddr::V6(ipv6);
        let ip_for_host = IpForHost::from(ip_addr);
        assert_eq!(ip_for_host, IpForHost::V6(ipv6));
    }

    #[test]
    fn from_tuple_v4_only() {
        let ipv4 = Ipv4Addr::new(192, 168, 1, 1);
        let ip_for_host = IpForHost::try_from((Some(ipv4), None)).unwrap();
        assert_eq!(ip_for_host, IpForHost::V4(ipv4));
    }

    #[test]
    fn from_tuple_v6_only() {
        let ipv6 = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        let ip_for_host = IpForHost::try_from((None, Some(ipv6))).unwrap();
        assert_eq!(ip_for_host, IpForHost::V6(ipv6));
    }

    #[test]
    fn from_tuple_both() {
        let ipv4 = Ipv4Addr::new(192, 168, 1, 1);
        let ipv6 = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        let ip_for_host = IpForHost::try_from((Some(ipv4), Some(ipv6))).unwrap();
        assert_eq!(ip_for_host, IpForHost::V4andV6((ipv4, ipv6)));
    }

    #[test]
    fn display_v4() {
        let ip = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1));
        assert_eq!(format!("{ip}"), "192.168.1.1");
    }

    #[test]
    fn display_v6() {
        let ip = IpForHost::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        assert_eq!(format!("{ip}"), "2001:db8::1");
    }

    #[test]
    fn display_v4andv6() {
        let ip = IpForHost::V4andV6((
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
        ));
        assert_eq!(format!("{ip}"), "192.168.1.1\n2001:db8::1");
    }

    #[test]
    fn hash_consistency() {
        use std::collections::HashMap;

        let ip1 = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip2 = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1));

        let mut map = HashMap::new();
        map.insert(ip1, "value");
        assert_eq!(map.get(&ip2), Some(&"value"));
    }
}
