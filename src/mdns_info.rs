use std::{collections::HashMap, fmt::Display, net::IpAddr};

#[derive(Debug)]
pub struct AccumulatedMdnsInfo {
    collection: HashMap<IpAddr, MdnsInfo>,
}

impl AccumulatedMdnsInfo {
    pub fn new() -> Self {
        Self {
            collection: HashMap::<IpAddr, MdnsInfo>::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.collection.len()
    }

    pub fn collection(&self) -> &HashMap<IpAddr, MdnsInfo> {
        &self.collection
    }

    pub fn insert(&mut self, mdns_info: MdnsInfo) {
        match self.collection.get_mut(&mdns_info.ip) {
            Some(info) => {
                info.seen_count += 1;
                for name in mdns_info.names() {
                    if !info.names().contains(name) {
                        info.names.push(name.to_owned());
                    }
                }
            }
            None => _ = self.collection.insert(mdns_info.ip, mdns_info),
        }
    }

    pub fn get_as_str_vec(&self) -> Vec<String> {
        let mut str_vec = vec![];
        for (_ip, info) in self.collection.iter() {
            str_vec.push(info.to_string());
        }
        str_vec
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct MdnsInfo {
    pub(crate) ip: IpAddr,
    pub(crate) names: Vec<String>,
    pub(crate) extra: Option<String>,
    pub(crate) seen_count: u64,
}

impl MdnsInfo {
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
}

impl Display for MdnsInfo {
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
