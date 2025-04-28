use std::{
    fmt::Display,
    net::IpAddr,
    time::{Duration, Instant},
};

use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LastKnownStatus {
    Online,
    Offline,
}

pub(crate) mod db;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct IpInfo {
    pub(crate) ip: IpAddr,
    pub(crate) names: Vec<String>,
    pub(crate) extra: Option<String>,
    pub(crate) last_known_status: LastKnownStatus,
    pub(crate) seen_count: u64,
    pub(crate) last_updated: Instant,
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
            last_known_status: LastKnownStatus::Online,
            seen_count: 1,
            last_updated: Instant::now(),
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

    pub(crate) fn set_last_known_status(&mut self, status: LastKnownStatus) {
        self.last_known_status = status;
        self.set_last_updated_now();
    }

    pub(crate) fn is_offline(&self) -> bool {
        self.last_known_status == LastKnownStatus::Offline
    }

    pub(crate) fn update_packets_seen(&mut self) {
        self.seen_count += 1;
    }

    pub(crate) fn set_last_updated_now(&mut self) {
        self.last_updated = Instant::now();
    }

    pub(crate) fn last_updated(&self) -> Duration {
        self.last_updated.elapsed()
    }

    pub(crate) fn updated_within_secs(&self, secs: u16) -> bool {
        self.last_updated().as_secs() < secs.into()
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
