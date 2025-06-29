use std::{
    fmt::Display,
    net::IpAddr,
    time::{Duration, Instant},
};

use mds_dns_sd::ServiceInfo;
use mds_util::host_up::ReachedBy;
use unicode_width::UnicodeWidthStr;

use crate::service::ServiceInstance;

pub mod db;
pub mod service;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LastKnownStatus {
    Online,
    Offline,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IpInfo {
    pub(crate) ip: IpAddr,
    pub(crate) reached_by: Option<ReachedBy>,
    pub(crate) names: Vec<String>,
    pub(crate) service_instances: Option<Vec<ServiceInstance>>,
    pub(crate) last_known_status: LastKnownStatus,
    pub(crate) seen_count: u64,
    pub(crate) last_updated: Instant,
}

impl IpInfo {
    pub fn ref_array(&self) -> [String; 4] {
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
            reached_by: None,
            names: vec![],
            service_instances: None,
            last_known_status: LastKnownStatus::Online,
            seen_count: 1,
            last_updated: Instant::now(),
        }
    }

    pub fn reached_with(mut self, method: ReachedBy) -> Self {
        self.reached_by = Some(method);
        self
    }

    pub fn reached_by(&self) -> Option<ReachedBy> {
        self.reached_by
    }

    /// Overwrite the 'reached by' information
    pub fn set_reached_by(&mut self, method: ReachedBy) {
        debug_assert_ne!(
            self.reached_by,
            Some(ReachedBy::EchoReply),
            "Not allowed to overwrite 'reached by' information if it's ping"
        );
        debug_assert!(
            !matches!(self.reached_by, Some(ReachedBy::Port(_))),
            "Not allowed to overwrite 'reached by' information if it's TCP port"
        );
        self.reached_by = Some(method);
    }

    pub fn ip(&self) -> IpAddr {
        self.ip
    }

    pub fn set_names(&mut self, names: Vec<String>) {
        self.names = names
    }

    pub fn names(&self) -> &[String] {
        self.names.as_slice()
    }

    pub fn add_name(&mut self, name: String) {
        self.names.push(name);
    }

    pub fn sort_names(&mut self) {
        self.names.sort();
    }

    pub fn dedup_names(&mut self) {
        self.names.dedup();
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

    pub fn services(&self) -> Option<&[ServiceInstance]> {
        self.service_instances.as_deref()
    }

    pub fn drain_services(
        &mut self,
    ) -> std::iter::Flatten<std::option::IntoIter<Vec<ServiceInstance>>> {
        self.service_instances.take().into_iter().flatten()
    }

    pub fn seen_count(&self) -> u64 {
        self.seen_count
    }

    pub fn incr_seen_count(&mut self) {
        self.seen_count += 1;
    }

    pub fn max_name_unicode_width(&self) -> u16 {
        let mut max = 0;
        for name in &self.names {
            let unicode_width = name.width();
            if max < unicode_width {
                max = unicode_width;
            }
        }
        max as u16
    }

    pub fn max_service_instance_unicode_width(&self) -> u16 {
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
    pub fn contains(&self, pattern: &str) -> bool {
        self.ip.to_string().contains(pattern) || self.names().iter().any(|n| n.contains(pattern))
    }

    pub fn matches_status(&self, status: LastKnownStatus) -> bool {
        self.last_known_status == status
    }

    pub fn set_last_known_status(&mut self, status: LastKnownStatus) {
        self.last_known_status = status;
        self.set_last_updated_now();
    }

    pub fn is_offline(&self) -> bool {
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

    pub fn updated_within_secs(&self, secs: u16) -> bool {
        self.last_updated().as_secs() < secs.into()
    }

    /// Returns whether or not an update was applied
    pub fn update_with_service_instance(&mut self, service: ServiceInstance) -> bool {
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
            reached_by: Some(ReachedBy::Mdns),
            names: vec![],
            service_instances: Some(vec![service_instance]),
            last_known_status: LastKnownStatus::Online,
            seen_count: 1,
            last_updated: Instant::now(),
        }
    }
}
