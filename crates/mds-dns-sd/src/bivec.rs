#![allow(
    dead_code,
    reason = "Basic operations are needed for tests, and they are generic container operations so there's no real benefit to removing them"
)]

use mds_util::prelude::DnsName;
use std::net::IpAddr;

#[derive(Debug, Clone)]
pub struct IpHostnamePair {
    pub ip: IpAddr,
    pub hostname: DnsName,
}

#[derive(Debug, Default)]
pub struct IpHostnameLookupVec {
    pairs: Vec<IpHostnamePair>,
}

impl IpHostnameLookupVec {
    pub fn new() -> Self {
        Self { pairs: Vec::new() }
    }

    /// Insert a new IP-hostname pair.
    pub fn insert(&mut self, ip: IpAddr, hostname: DnsName) {
        if !self.contains_pair(&ip, &hostname) {
            self.pairs.push(IpHostnamePair { ip, hostname });
        }
    }

    /// Get all hostnames associated with an IP
    pub fn get_hostnames_by_ip(&self, ip: &IpAddr) -> Vec<&DnsName> {
        self.pairs
            .iter()
            .filter(|pair| &pair.ip == ip)
            .map(|pair| &pair.hostname)
            .collect()
    }

    /// Get all IPs associated with a hostname
    pub fn get_ips_by_hostname(&self, hostname: &DnsName) -> Vec<&IpAddr> {
        self.pairs
            .iter()
            .filter(|pair| pair.hostname.matches(hostname))
            .map(|pair| &pair.ip)
            .collect()
    }

    /// Remove all pairs with the given IP
    pub fn remove_all_by_ip(&mut self, ip: &IpAddr) -> Vec<IpHostnamePair> {
        let mut removed = Vec::new();
        self.pairs.retain(|pair| {
            if &pair.ip == ip {
                removed.push(pair.clone());
                false
            } else {
                true
            }
        });
        removed
    }

    /// Remove all pairs with the given hostname
    pub fn remove_all_by_hostname(&mut self, hostname: &DnsName) -> Vec<IpHostnamePair> {
        let mut removed = Vec::new();
        self.pairs.retain(|pair| {
            if pair.hostname.matches(hostname) {
                removed.push(pair.clone());
                false
            } else {
                true
            }
        });
        removed
    }

    /// Check if a specific IP-hostname pair exists
    pub fn contains_pair(&self, ip: &IpAddr, hostname: &DnsName) -> bool {
        self.pairs
            .iter()
            .any(|pair| &pair.ip == ip && pair.hostname.matches(hostname))
    }

    /// Check if any pair with this IP exists
    pub fn contains_ip(&self, ip: &IpAddr) -> bool {
        self.pairs.iter().any(|pair| &pair.ip == ip)
    }

    /// Check if any pair with this hostname exists
    pub fn contains_hostname(&self, hostname: &DnsName) -> bool {
        self.pairs
            .iter()
            .any(|pair| pair.hostname.matches(hostname))
    }

    /// Get all unique IPs
    pub fn get_all_ips(&self) -> Vec<IpAddr> {
        let mut seen = Vec::new();
        for pair in &self.pairs {
            if !seen.contains(&pair.ip) {
                seen.push(pair.ip);
            }
        }
        seen
    }

    /// Get all unique hostnames
    pub fn get_all_hostnames(&self) -> Vec<&DnsName> {
        let mut seen: Vec<&DnsName> = Vec::new();
        for pair in &self.pairs {
            if !seen.iter().any(|s| s.matches(&pair.hostname)) {
                seen.push(&pair.hostname);
            }
        }
        seen
    }

    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    pub fn clear(&mut self) {
        self.pairs.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_multiple_hostnames_per_ip() {
        let mut lookup = IpHostnameLookupVec::new();
        let ip = IpAddr::from_str("192.168.1.1").unwrap();

        // Insert multiple hostnames for same IP
        lookup.insert(ip, DnsName::new("server1.local"));
        lookup.insert(ip, DnsName::new("web.local"));
        lookup.insert(ip, DnsName::new("api.local"));

        let hostnames = lookup.get_hostnames_by_ip(&ip);
        assert_eq!(hostnames.len(), 3);
        assert!(hostnames.iter().any(|h| h.matches_str("server1.local")));
        assert!(hostnames.iter().any(|h| h.matches_str("web.local")));
        assert!(hostnames.iter().any(|h| h.matches_str("api.local")));
    }

    #[test]
    fn test_multiple_ips_per_hostname() {
        let mut lookup = IpHostnameLookupVec::new();
        let ip1 = IpAddr::from_str("192.168.1.1").unwrap();
        let ip2 = IpAddr::from_str("192.168.1.2").unwrap();
        let ip3 = IpAddr::from_str("10.0.0.1").unwrap();

        // Insert multiple IPs for same hostname
        lookup.insert(ip1, DnsName::new("loadbalancer.local"));
        lookup.insert(ip2, DnsName::new("loadbalancer.local"));
        lookup.insert(ip3, DnsName::new("loadbalancer.local"));

        let ips = lookup.get_ips_by_hostname(&DnsName::new("loadbalancer.local"));
        assert_eq!(ips, [&ip1, &ip2, &ip3]);
    }

    #[test]
    fn test_no_duplicate_pairs() {
        let mut lookup = IpHostnameLookupVec::new();
        let ip = IpAddr::from_str("192.168.1.1").unwrap();

        lookup.insert(ip, DnsName::new("test.local"));
        assert_eq!(lookup.len(), 1);

        // Duplicate insert should not grow, regardless of spelling
        lookup.insert(ip, DnsName::new("test.local"));
        assert_eq!(lookup.len(), 1);
        lookup.insert(ip, DnsName::new("Test.Local."));
        assert_eq!(lookup.len(), 1);
    }
}
