#![allow(
    dead_code,
    reason = "Basic operations are needed for tests, and they are generic container operations so there's no real benefit to removing them"
)]

use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpHostnamePair {
    pub ip: IpAddr,
    pub hostname: String,
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
    pub fn insert(&mut self, ip: IpAddr, hostname: String) {
        let new_pair = IpHostnamePair { ip, hostname };

        // Check if exact pair already exists
        if !self.pairs.contains(&new_pair) {
            self.pairs.push(new_pair);
        }
    }

    /// Get all hostnames associated with an IP
    pub fn get_hostnames_by_ip(&self, ip: &IpAddr) -> Vec<&String> {
        self.pairs
            .iter()
            .filter(|pair| &pair.ip == ip)
            .map(|pair| &pair.hostname)
            .collect()
    }

    /// Get all IPs associated with a hostname
    pub fn get_ips_by_hostname(&self, hostname: &str) -> Vec<&IpAddr> {
        self.pairs
            .iter()
            .filter(|pair| pair.hostname == hostname)
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
    pub fn remove_all_by_hostname(&mut self, hostname: &str) -> Vec<IpHostnamePair> {
        let mut removed = Vec::new();
        self.pairs.retain(|pair| {
            if pair.hostname == hostname {
                removed.push(pair.clone());
                false
            } else {
                true
            }
        });
        removed
    }

    /// Check if a specific IP-hostname pair exists
    pub fn contains_pair(&self, ip: &IpAddr, hostname: &str) -> bool {
        self.pairs
            .iter()
            .any(|pair| &pair.ip == ip && pair.hostname == hostname)
    }

    /// Check if any pair with this IP exists
    pub fn contains_ip(&self, ip: &IpAddr) -> bool {
        self.pairs.iter().any(|pair| &pair.ip == ip)
    }

    /// Check if any pair with this hostname exists
    pub fn contains_hostname(&self, hostname: &str) -> bool {
        self.pairs.iter().any(|pair| pair.hostname == hostname)
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
    pub fn get_all_hostnames(&self) -> Vec<&String> {
        let mut seen: Vec<&String> = Vec::new();
        for pair in &self.pairs {
            if !seen.contains(&&pair.hostname) {
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
        lookup.insert(ip, "server1.local".to_string());
        lookup.insert(ip, "web.local".to_string());
        lookup.insert(ip, "api.local".to_string());

        let hostnames = lookup.get_hostnames_by_ip(&ip);
        assert_eq!(hostnames.len(), 3);
        assert!(hostnames.contains(&&"server1.local".to_string()));
        assert!(hostnames.contains(&&"web.local".to_string()));
        assert!(hostnames.contains(&&"api.local".to_string()));
    }

    #[test]
    fn test_multiple_ips_per_hostname() {
        let mut lookup = IpHostnameLookupVec::new();
        let ip1 = IpAddr::from_str("192.168.1.1").unwrap();
        let ip2 = IpAddr::from_str("192.168.1.2").unwrap();
        let ip3 = IpAddr::from_str("10.0.0.1").unwrap();

        // Insert multiple IPs for same hostname
        lookup.insert(ip1, "loadbalancer.local".to_string());
        lookup.insert(ip2, "loadbalancer.local".to_string());
        lookup.insert(ip3, "loadbalancer.local".to_string());

        let ips = lookup.get_ips_by_hostname("loadbalancer.local");
        assert!(ips.contains(&&ip1));
        assert!(ips.contains(&&ip2));
        assert!(ips.contains(&&ip3));
        assert_eq!(ips.len(), 3);
    }

    #[test]
    fn test_no_duplicate_pairs() {
        let mut lookup = IpHostnameLookupVec::new();
        let ip = IpAddr::from_str("192.168.1.1").unwrap();

        lookup.insert(ip, "test.local".to_string());
        assert_eq!(lookup.len(), 1);

        // Duplicate insert should not grow
        lookup.insert(ip, "test.local".to_string());
        assert_eq!(lookup.len(), 1);
    }
}
