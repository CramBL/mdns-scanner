use std::{collections::HashMap, net::IpAddr};

use super::IpInfo;

#[derive(Debug)]
pub struct IpDb {
    ip_info: HashMap<IpAddr, IpInfo>,
}

impl IpDb {
    pub fn new() -> Self {
        Self {
            ip_info: HashMap::<IpAddr, IpInfo>::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.ip_info.len()
    }

    pub fn insert(&mut self, ip_info: IpInfo) {
        match self.ip_info.get_mut(&ip_info.ip) {
            Some(info) => {
                info.seen_count += 1;
                for name in ip_info.names() {
                    if !info.names().contains(name) {
                        info.names.push(name.to_owned());
                    }
                }
            }
            None => _ = self.ip_info.insert(ip_info.ip, ip_info),
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
}
