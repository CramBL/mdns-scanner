use std::{collections::HashMap, fmt::Display, net::IpAddr};

use unicode_width::UnicodeWidthStr;

#[derive(Debug)]
pub struct AccumulatedIpInfo {
    collection: HashMap<IpAddr, IpInfo>,
}

impl AccumulatedIpInfo {
    pub fn new() -> Self {
        Self {
            collection: HashMap::<IpAddr, IpInfo>::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.collection.len()
    }

    pub fn insert(&mut self, ip_info: IpInfo) {
        match self.collection.get_mut(&ip_info.ip) {
            Some(info) => {
                info.seen_count += 1;
                for name in ip_info.names() {
                    if !info.names().contains(name) {
                        info.names.push(name.to_owned());
                    }
                }
            }
            None => _ = self.collection.insert(ip_info.ip, ip_info),
        }
    }

    pub fn get_ip_info(&self, filter_pattern: Option<&str>) -> Vec<&IpInfo> {
        let mut ip_info_vec: Vec<&IpInfo> = self
            .collection
            .iter()
            .map(|(_ip, ip_info)| ip_info)
            .collect();
        ip_info_vec.sort_unstable_by_key(|a| a.ip());

        if let Some(pattern) = filter_pattern {
            ip_info_vec.retain(|i| i.contains(pattern));
        }
        ip_info_vec
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct IpInfo {
    pub(crate) ip: IpAddr,
    pub(crate) names: Vec<String>,
    pub(crate) extra: Option<String>,
    pub(crate) seen_count: u64,
}

impl IpInfo {
    pub(crate) fn ref_array(&self) -> [String; 3] {
        [
            self.ip.to_string(),
            self.names_multiline_string().to_owned(),
            self.seen_count.to_string(),
        ]
    }

    pub fn from_ip(ip: IpAddr) -> Self {
        Self {
            ip,
            names: vec![],
            extra: None,
            seen_count: 1,
        }
    }

    pub fn ip(&self) -> IpAddr {
        self.ip
    }

    pub fn names(&self) -> &[String] {
        self.names.as_slice()
    }

    pub fn names_multiline_string(&self) -> String {
        let mut names_str = String::new();
        for n in &self.names {
            names_str.push_str(n);
            names_str.push('\n');
        }
        names_str
    }

    pub fn seen_count(&self) -> u64 {
        self.seen_count
    }

    pub(crate) fn max_name_unicode_width(&self) -> u16 {
        let mut max = 0;
        for name in &self.names {
            let unicode_width = name.width();
            if max < unicode_width {
                max = unicode_width;
            }
        }
        max as u16
    }

    /// Filtering function
    pub(crate) fn contains(&self, pattern: &str) -> bool {
        self.ip.to_string().contains(pattern) || self.names().iter().any(|n| n.contains(pattern))
    }
}

impl Display for IpInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} (seen {} times)",
            self.ip,
            self.names_multiline_string(),
            self.seen_count
        )
    }
}
