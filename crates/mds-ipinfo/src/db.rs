use std::time::Duration;

use super::{IpInfo, LastKnownStatus};
use crate::IpForHost;

#[derive(Debug, Default)]
pub struct IpDb {
    ip_info: Vec<IpInfo>,
}

impl IpDb {
    pub fn len(&self) -> usize {
        self.ip_info.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ip_info.is_empty()
    }

    pub fn insert(&mut self, ip_info: IpInfo) {
        let new_ip = ip_info.ip();
        let mut merged_info = ip_info;

        // Find indices of entries that share the IP
        let mut matching_indices = Vec::new();
        for (i, info) in self.ip_info.iter().enumerate() {
            if info.ip().shares_ip_with(&new_ip) {
                matching_indices.push(i);
            }
        }

        // Remove and merge the matching entries
        for i in matching_indices.iter().rev() {
            let mut old_info = self.ip_info.swap_remove(*i);
            old_info.merge(merged_info);
            merged_info = old_info;
        }

        merged_info.set_last_updated_now();
        self.ip_info.push(merged_info);
    }

    pub fn update_packets_seen(&mut self, ip: IpForHost, rtt: Option<Duration>) {
        if let Some(info) = self.get_mut(ip) {
            info.update_packets_seen();
            info.set_last_known_status((LastKnownStatus::Online, rtt));
        }
    }

    pub fn update_last_known_status(
        &mut self,
        ip: IpForHost,
        (status, rtt): (LastKnownStatus, Option<Duration>),
    ) {
        if let Some(info) = self.get_mut(ip) {
            info.set_last_known_status((status, rtt));
        }
    }

    fn get_mut(&mut self, ip: IpForHost) -> Option<&mut IpInfo> {
        match ip {
            IpForHost::V4(ipv4) => self
                .ip_info
                .iter_mut()
                .find(|i| i.ip() == IpForHost::V4(ipv4)),
            IpForHost::V6(ipv6) => self
                .ip_info
                .iter_mut()
                .find(|i| i.ip() == IpForHost::V6(ipv6)),
            IpForHost::V4andV6((ipv4, ipv6)) => {
                let keys = [
                    IpForHost::V4andV6((ipv4, ipv6)),
                    IpForHost::V4(ipv4),
                    IpForHost::V6(ipv6),
                ];

                for info in &mut self.ip_info {
                    if keys.iter().any(|key| info.ip() == *key) {
                        return Some(info);
                    }
                }
                None
            }
        }
    }

    pub fn get_ip_info(&self, filter_pattern: Option<&str>) -> Vec<&IpInfo> {
        let mut results: Vec<&IpInfo> = self.ip_info.iter().collect();
        results.sort_unstable_by_key(|i| i.ip());

        if let Some(pattern) = filter_pattern {
            results.retain(|info| info.contains(pattern));
        }

        results
    }

    pub fn clear(&mut self) {
        self.ip_info.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::time::Duration;

    fn create_test_ip_info(ip: IpAddr) -> IpInfo {
        IpInfo::from_ip(ip)
    }

    fn create_ip_info_with_names(ip: IpAddr, names: Vec<String>) -> IpInfo {
        let mut info = IpInfo::from_ip(ip);
        info.set_names(names);
        info
    }

    #[test]
    fn insert_different_ips() {
        let mut db = IpDb::default();
        let ip1 = create_test_ip_info(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)));
        let ip2 = create_test_ip_info(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)));

        db.insert(ip1);
        db.insert(ip2);

        assert_eq!(db.len(), 2);
    }

    #[test]
    fn insert_same_ip_merges() {
        let mut db = IpDb::default();
        let ip_addr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let mut ip1 = create_test_ip_info(ip_addr);
        ip1.seen_count = 5;
        let mut ip2 = create_test_ip_info(ip_addr);
        ip2.seen_count = 3;

        db.insert(ip1);
        db.insert(ip2);

        assert_eq!(db.len(), 1);
        let results = db.get_ip_info(None);
        assert_eq!(results[0].seen_count(), 8);
    }

    #[test]
    fn insert_shared_ip_v4_and_v4andv6_merges() {
        let mut db = IpDb::default();
        let ipv4 = Ipv4Addr::new(192, 168, 1, 1);
        let ipv6 = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);

        let ip1 = create_test_ip_info(IpAddr::V4(ipv4));
        let mut ip2 = create_test_ip_info(IpAddr::V4(ipv4));
        ip2.ip = IpForHost::V4andV6((ipv4, ipv6));

        db.insert(ip1);
        db.insert(ip2);

        assert_eq!(db.len(), 1);
        let results = db.get_ip_info(None);
        assert_eq!(results[0].ip(), IpForHost::V4andV6((ipv4, ipv6)));
    }

    #[test]
    fn insert_merges_names() {
        let mut db = IpDb::default();
        let ip_addr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip1 = create_ip_info_with_names(ip_addr, vec!["host1.local".to_string()]);
        let ip2 = create_ip_info_with_names(ip_addr, vec!["host2.local".to_string()]);

        db.insert(ip1);
        db.insert(ip2);

        assert_eq!(db.len(), 1);
        let results = db.get_ip_info(None);
        let names = results[0].names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"host1.local".to_string()));
        assert!(names.contains(&"host2.local".to_string()));
    }

    #[test]
    fn update_packets_seen_existing_ip() {
        let mut db = IpDb::default();
        let ip_for_host = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip_addr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip_info = create_test_ip_info(ip_addr);

        db.insert(ip_info);
        db.update_packets_seen(ip_for_host, Some(Duration::from_millis(10)));

        let results = db.get_ip_info(None);
        assert_eq!(results[0].seen_count(), 2);
        assert_eq!(results[0].last_known_status, LastKnownStatus::Online);
    }

    #[test]
    fn update_packets_seen_nonexistent_ip() {
        let mut db = IpDb::default();
        let ip_for_host = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1));

        db.update_packets_seen(ip_for_host, None);

        assert_eq!(db.len(), 0);
    }

    #[test]
    fn update_last_known_status_existing_ip() {
        let mut db = IpDb::default();
        let ip_for_host = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip_addr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip_info = create_test_ip_info(ip_addr);

        db.insert(ip_info);
        db.update_last_known_status(ip_for_host, (LastKnownStatus::Offline, None));

        let results = db.get_ip_info(None);
        assert_eq!(results[0].last_known_status, LastKnownStatus::Offline);
    }

    #[test]
    fn update_last_known_status_nonexistent_ip() {
        let mut db = IpDb::default();
        let ip_for_host = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1));

        db.update_last_known_status(ip_for_host, (LastKnownStatus::Offline, None));

        assert_eq!(db.len(), 0);
    }

    #[test]
    fn get_mut_v4_exact_match() {
        let mut db = IpDb::default();
        let ip_addr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip_info = create_test_ip_info(ip_addr);

        db.insert(ip_info);

        let ip_for_host = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1));
        let result = db.get_mut(ip_for_host);
        assert!(result.is_some());
    }

    #[test]
    fn get_mut_v6_exact_match() {
        let mut db = IpDb::default();
        let ip_addr = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        let ip_info = create_test_ip_info(ip_addr);

        db.insert(ip_info);

        let ip_for_host = IpForHost::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        let result = db.get_mut(ip_for_host);
        assert!(result.is_some());
    }

    #[test]
    fn get_mut_v4andv6_exact_match() {
        let mut db = IpDb::default();
        let ipv4 = Ipv4Addr::new(192, 168, 1, 1);
        let ipv6 = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        let mut ip_info = create_test_ip_info(IpAddr::V4(ipv4));
        ip_info.ip = IpForHost::V4andV6((ipv4, ipv6));

        db.insert(ip_info);

        let ip_for_host = IpForHost::V4andV6((ipv4, ipv6));
        let result = db.get_mut(ip_for_host);
        assert!(result.is_some());
    }

    #[test]
    fn get_mut_v4andv6_finds_v4_match() {
        let mut db = IpDb::default();
        let ipv4 = Ipv4Addr::new(192, 168, 1, 1);
        let ipv6 = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        let ip_info = create_test_ip_info(IpAddr::V4(ipv4));

        db.insert(ip_info);

        let ip_for_host = IpForHost::V4andV6((ipv4, ipv6));
        let result = db.get_mut(ip_for_host);
        assert!(result.is_some());
    }

    #[test]
    fn get_mut_v4andv6_finds_v6_match() {
        let mut db = IpDb::default();
        let ipv4 = Ipv4Addr::new(192, 168, 1, 1);
        let ipv6 = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        let ip_info = create_test_ip_info(IpAddr::V6(ipv6));

        db.insert(ip_info);

        let ip_for_host = IpForHost::V4andV6((ipv4, ipv6));
        let result = db.get_mut(ip_for_host);
        assert!(result.is_some());
    }

    #[test]
    fn get_mut_no_match() {
        let mut db = IpDb::default();
        let ip_info = create_test_ip_info(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)));

        db.insert(ip_info);

        let ip_for_host = IpForHost::V4(Ipv4Addr::new(192, 168, 1, 2));
        let result = db.get_mut(ip_for_host);
        assert!(result.is_none());
    }

    #[test]
    fn get_ip_info_returns_sorted() {
        let mut db = IpDb::default();
        let ip1 = create_test_ip_info(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)));
        let ip2 = create_test_ip_info(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)));
        let ip3 = create_test_ip_info(IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)));

        db.insert(ip1);
        db.insert(ip2);
        db.insert(ip3);

        let results = db.get_ip_info(None);
        assert_eq!(results.len(), 3);
        assert_eq!(
            results[0].ip(),
            IpForHost::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1))
        );
        assert_eq!(
            results[1].ip(),
            IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1))
        );
        assert_eq!(
            results[2].ip(),
            IpForHost::V4(Ipv4Addr::new(192, 168, 1, 2))
        );
    }

    #[test]
    fn get_ip_info_with_filter() {
        let mut db = IpDb::default();
        let ip1 = create_ip_info_with_names(
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            vec!["test.local".to_string()],
        );
        let ip2 = create_ip_info_with_names(
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
            vec!["other.local".to_string()],
        );

        db.insert(ip1);
        db.insert(ip2);

        let results = db.get_ip_info(Some("test"));
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].ip(),
            IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1))
        );
    }

    #[test]
    fn get_ip_info_filter_by_ip() {
        let mut db = IpDb::default();
        let ip1 = create_test_ip_info(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)));
        let ip2 = create_test_ip_info(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));

        db.insert(ip1);
        db.insert(ip2);

        let results = db.get_ip_info(Some("192.168"));
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].ip(),
            IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1))
        );
    }

    /// A dual-stack mDNS entry (V4+V6) and an existing IPv4 scanner entry for
    /// the same host must merge into one row with one deduplicated hostname,
    /// even if one form has a trailing dot and the other does not.
    #[test]
    fn insert_deduplicates_trailing_dot_hostname_on_dual_stack_merge() {
        let mut db = IpDb::default();
        let ipv4 = Ipv4Addr::new(192, 168, 1, 1);
        let ipv6 = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);

        // Network scanner: reverse DNS, no trailing dot.
        let mut scanner_entry = create_test_ip_info(IpAddr::V4(ipv4));
        scanner_entry.set_names(vec!["hostname.local".to_owned()]);

        // mDNS/DNS-SD: dual-stack, absolute FQDN (trailing dot).
        let mut mdns_entry = IpInfo::from_host(IpForHost::V4andV6((ipv4, ipv6)));
        mdns_entry.set_names(vec!["hostname.local.".to_owned()]);

        db.insert(scanner_entry);
        db.insert(mdns_entry);

        let results = db.get_ip_info(None);
        assert_eq!(
            results.len(),
            1,
            "dual-stack mDNS entry should merge with existing IPv4 scanner entry"
        );
        let actual_names = results[0].names();
        assert_eq!(
            actual_names,
            &["hostname.local"],
            "expected exactly one name after dual-stack merge with/without trailing dot, got: {actual_names:?}"
        );
    }

    #[test]
    fn get_ip_info_no_filter() {
        let mut db = IpDb::default();
        let ip1 = create_test_ip_info(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)));
        let ip2 = create_test_ip_info(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)));

        db.insert(ip1);
        db.insert(ip2);

        let results = db.get_ip_info(None);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn clear_empties_db() {
        let mut db = IpDb::default();
        let ip_info = create_test_ip_info(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)));

        db.insert(ip_info);
        assert_eq!(db.len(), 1);

        db.clear();
        assert_eq!(db.len(), 0);
        assert!(db.is_empty());
    }

    #[test]
    fn insert_multiple_merges_into_single_entry() {
        let mut db = IpDb::default();
        let ipv4 = Ipv4Addr::new(192, 168, 1, 1);

        let ip1 = create_test_ip_info(IpAddr::V4(ipv4));
        let mut ip2 = create_test_ip_info(IpAddr::V4(ipv4));
        ip2.seen_count = 5;
        let mut ip3 = create_test_ip_info(IpAddr::V4(ipv4));
        ip3.seen_count = 3;

        db.insert(ip1);
        db.insert(ip2);
        db.insert(ip3);

        assert_eq!(db.len(), 1);
        let results = db.get_ip_info(None);
        assert_eq!(results[0].seen_count(), 9);
    }

    #[test]
    fn insert_preserves_order_for_different_ips() {
        let mut db = IpDb::default();

        for i in 1..=5 {
            let ip_info = create_test_ip_info(IpAddr::V4(Ipv4Addr::new(192, 168, 1, i)));
            db.insert(ip_info);
        }

        assert_eq!(db.len(), 5);
        for (i, info) in db.get_ip_info(None).iter().enumerate().take(5) {
            assert_eq!(
                info.ip(),
                IpForHost::V4(Ipv4Addr::new(192, 168, 1, (i + 1) as u8))
            );
        }
    }

    #[test]
    fn stress_test_many_inserts_and_merges() {
        let mut db = IpDb::default();
        let num_hosts = 100;

        for i in 0..num_hosts {
            let ipv4 = Ipv4Addr::new(192, 168, 1, (i % 254 + 1) as u8);
            let mut info = IpInfo::from_ip(IpAddr::V4(ipv4));
            info.add_name(format!("host{}.local", i));
            db.insert(info);
        }

        // All should be there, unique by IPv4
        assert_eq!(db.len(), num_hosts as usize);

        // insert again as V6
        for i in 0..num_hosts {
            let ipv6 = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, (i + 1) as u16);
            let mut info = IpInfo::from_ip(IpAddr::V6(ipv6));
            info.add_name(format!("host{}.local", i));
            db.insert(info);
        }

        // they don't share IPs yet, so 2*num_hosts
        assert_eq!(db.len(), (2 * num_hosts) as usize);

        // insert as V4andV6 to trigger merges
        for i in 0..num_hosts {
            let ipv4 = Ipv4Addr::new(192, 168, 1, (i % 254 + 1) as u8);
            let ipv6 = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, (i + 1) as u16);
            let mut info = IpInfo::from_host(IpForHost::V4andV6((ipv4, ipv6)));
            info.add_name(format!("host{}.local", i));
            db.insert(info);
        }

        // All should have been merged into num_hosts entries
        assert_eq!(db.len(), num_hosts as usize);
    }
}
