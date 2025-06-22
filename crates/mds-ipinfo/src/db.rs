use std::{collections::HashMap, net::IpAddr};

use super::{IpInfo, LastKnownStatus};

#[derive(Debug, Default)]
pub struct IpDb {
    ip_info: HashMap<IpAddr, IpInfo>,
}

impl IpDb {
    pub fn len(&self) -> usize {
        self.ip_info.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ip_info.is_empty()
    }

    pub fn insert(&mut self, mut ip_info: IpInfo) {
        ip_info.set_last_updated_now();
        _ = self.ip_info.insert(ip_info.ip, ip_info);
    }

    pub fn update_packets_seen(&mut self, ip: IpAddr) {
        if let Some(ip_info) = self.ip_info.get_mut(&ip) {
            ip_info.update_packets_seen();
            ip_info.set_last_known_status(LastKnownStatus::Online);
        }
    }

    pub fn update_last_known_status(&mut self, ip: IpAddr, status: LastKnownStatus) {
        if let Some(ip_info) = self.ip_info.get_mut(&ip) {
            ip_info.set_last_known_status(status);
        }
    }

    pub fn get_ip_info(&self, filter_pattern: Option<&str>) -> Vec<&IpInfo> {
        let mut ip_info_vec: Vec<&IpInfo> = self.ip_info.values().collect::<Vec<_>>();
        ip_info_vec.sort_unstable_by_key(|a| a.ip());

        if let Some(pattern) = filter_pattern {
            ip_info_vec.retain(|i| i.contains(pattern));
        }
        ip_info_vec
    }

    pub fn clear(&mut self) {
        self.ip_info.clear();
    }
}
