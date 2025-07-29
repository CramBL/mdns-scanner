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
            let old_info = self.ip_info.swap_remove(*i);
            merged_info.merge(old_info);
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
