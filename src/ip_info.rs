use std::{
    fmt::Display,
    net::IpAddr,
    time::{Duration, Instant},
};

use unicode_width::UnicodeWidthStr;

use crate::dns_sd::ServiceInfo;

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
    pub(crate) service_instances: Option<Vec<ServiceInstance>>,
    pub(crate) last_known_status: LastKnownStatus,
    pub(crate) seen_count: u64,
    pub(crate) last_updated: Instant,
}

impl IpInfo {
    pub(crate) fn ref_array(&self) -> [String; 4] {
        [
            self.ip.to_string(),
            self.names_multiline().clone(),
            self.seen_count.to_string(),
            self.service_instances_multiline(),
        ]
    }

    pub fn from_ip(ip: IpAddr) -> Self {
        Self {
            ip,
            names: vec![],
            service_instances: None,
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

    fn names_multiline(&self) -> String {
        let mut names_str = String::new();
        for n in &self.names {
            names_str.push_str(n);
            names_str.push('\n');
        }
        names_str
    }

    fn service_instances_multiline(&self) -> String {
        let mut services_str = String::new();
        for s in self.service_instances.iter().flatten() {
            services_str.push_str(&s.to_string());
            services_str.push('\n');
        }
        services_str
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

    pub(crate) fn max_service_instance_unicode_width(&self) -> u16 {
        let mut max = 0;
        for service in self.service_instances.iter().flatten() {
            let unicode_width = service.to_string().width();
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

    /// Returns whether or not an update was applied
    pub(crate) fn update_with_service_instance(&mut self, service: ServiceInstance) -> bool {
        for s in self.service_instances.iter_mut().flatten() {
            if s.name == service.name {
                if *s == service {
                    return false;
                } else {
                    debug_assert_eq!(s._type, service._type, "mismatched service types");
                    debug_assert_eq!(s.port, service.port, "mismatched service port");
                    debug_assert_eq!(s.hostname, service.hostname, "mismatched service hostname");
                    if let Some(txt) = service.txt {
                        if let Some(mut s_txt) = s.txt.take() {
                            //
                            for t in txt {
                                if !s_txt.contains(&t) {
                                    s_txt.push(t);
                                }
                            }
                            s.txt = Some(s_txt);
                        } else {
                            s.txt = Some(txt);
                        }
                    }
                    return true;
                }
            }
        }
        if let Some(instances) = &mut self.service_instances {
            instances.push(service);
        } else {
            self.service_instances = Some(vec![service]);
        }
        true
    }
}

impl Display for IpInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} (seen {} times)",
            self.ip,
            self.names_multiline(),
            self.seen_count
        )
    }
}

impl From<ServiceInfo> for IpInfo {
    fn from(s: ServiceInfo) -> Self {
        let service_instance = ServiceInstance::new(s.name, s._type, Some(s.host), s.port, s.txt);

        IpInfo {
            ip: s.ip,
            names: vec![],
            service_instances: Some(vec![service_instance]),
            last_known_status: LastKnownStatus::Online,
            seen_count: 1,
            last_updated: Instant::now(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ServiceInstance {
    name: String,
    // Only applicable if it advertises an mDNS hostname by itself that doesn't match the hostname of the host at the IP its at
    pub(crate) hostname: Option<String>,
    _type: String,
    port: u16,
    txt: Option<Vec<String>>,
}

impl ServiceInstance {
    pub fn new(
        name: String,
        _type: String,
        hostname: Option<String>,
        port: u16,
        txt: Option<Vec<String>>,
    ) -> Self {
        Self {
            name,
            hostname,
            _type,
            port,
            txt,
        }
    }
}

impl Display for ServiceInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = &self.name;
        let host_opt = self
            .hostname
            .as_deref()
            .map(|h| format!(" @ {h}"))
            .unwrap_or_default();
        let port = self.port;
        let txt = self
            .txt
            .as_deref()
            .map(|t| t.join(", "))
            .unwrap_or_default();
        write!(f, "{name}{host_opt}:{port}\n{txt}")
    }
}
