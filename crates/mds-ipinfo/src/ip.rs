use std::{
    cmp::Ordering,
    fmt,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

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

impl From<(Option<Ipv4Addr>, Option<Ipv6Addr>)> for IpForHost {
    fn from((maybe_ipv4, maybe_ipv6): (Option<Ipv4Addr>, Option<Ipv6Addr>)) -> Self {
        match (maybe_ipv4, maybe_ipv6) {
            (None, None) => {
                unreachable!("Unsound condition")
            }
            (None, Some(ipv6)) => Self::V6(ipv6),
            (Some(ipv4), None) => Self::V4(ipv4),
            (Some(ipv4), Some(ipv6)) => Self::V4andV6((ipv4, ipv6)),
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
