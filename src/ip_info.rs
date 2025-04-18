use std::{fmt::Display, net::IpAddr};

use unicode_width::UnicodeWidthStr;

pub(crate) mod db;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
