use std::{
    fmt::{self, Display},
    net::IpAddr,
    time::{Duration, Instant},
};

use mds_util::host_up::{HostUpInfo, ReachedBy};
use mds_util::prelude::DnsName;
use unicode_width::UnicodeWidthStr;

use crate::{rtt_stats::RttStats, service::ServiceInstance};

pub mod db;
pub use ip::IpForHost;
pub mod ip;
pub mod rtt_stats;
pub mod service;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LastKnownStatus {
    Online,
    Offline,
}

impl fmt::Display for LastKnownStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LastKnownStatus::Online => write!(f, "Online"),
            LastKnownStatus::Offline => write!(f, "Offline"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IpInfo {
    pub ip: IpForHost,
    pub reached_by: Option<ReachedBy>,
    /// RTT on the first time the host was detected
    pub rtt: Option<RttStats>,
    names: Vec<DnsName>,
    pub service_instances: Option<Vec<ServiceInstance>>,
    pub last_known_status: LastKnownStatus,
    pub seen_count: u64,
    pub last_updated: Instant,
}

impl PartialEq for IpInfo {
    fn eq(&self, other: &Self) -> bool {
        // Destructure so a new field can't silently be left out of the comparison
        let Self {
            ip,
            reached_by,
            rtt,
            names,
            service_instances,
            last_known_status,
            seen_count,
            last_updated,
        } = self;
        *ip == other.ip
            && *reached_by == other.reached_by
            && *rtt == other.rtt
            && names.len() == other.names.len()
            && names.iter().zip(&other.names).all(|(a, b)| a.matches(b))
            && *service_instances == other.service_instances
            && *last_known_status == other.last_known_status
            && *seen_count == other.seen_count
            && *last_updated == other.last_updated
    }
}

impl Eq for IpInfo {}

impl IpInfo {
    /// Merges another `IpInfo` into this one.
    ///
    /// Prioritizes `self` over `other` for fields with no meaningful merge strategy
    pub fn merge(&mut self, other: Self) {
        let Self {
            ip,
            reached_by,
            rtt,
            names,
            service_instances,
            last_known_status,
            seen_count,
            last_updated,
        } = other;

        self.ip = self.ip.merge(ip);

        self.seen_count += seen_count;

        self.names.extend(names);
        self.names
            .sort_unstable_by(|a, b| a.canonical().cmp(b.canonical()));
        self.names.dedup_by(|a, b| a.matches(b));

        if let Some(other_services) = service_instances {
            for service in other_services {
                self.update_with_service_instance(service);
            }
        }

        self.post_process_services();

        // Merge the 'reached_by' status, preferring more reliable discovery methods.
        // EchoReply (ping) > Port (TCP) > Other.
        if let Some(other_reached_by) = reached_by {
            match self.reached_by {
                Some(ReachedBy::EchoReply) => {}
                Some(ReachedBy::Port(_)) => {
                    if matches!(other_reached_by, ReachedBy::EchoReply) {
                        self.reached_by = Some(other_reached_by);
                    }
                }
                _ => {
                    self.reached_by = Some(other_reached_by);
                }
            }
        }

        if let Some(cur_rtt) = &mut self.rtt {
            if let Some(other_rtt) = rtt {
                cur_rtt.merge(other_rtt);
            }
        } else {
            self.rtt = rtt;
        }

        if self.last_updated < last_updated {
            self.last_updated = last_updated;
            self.last_known_status = last_known_status;
        }
    }

    pub fn ref_array(&self) -> [String; 4] {
        [
            self.ip.to_string(),
            self.names_multiline(),
            self.seen_count.to_string(),
            self.service_instances_multiline(),
        ]
    }

    pub fn from_ip(ip: IpAddr) -> Self {
        Self::from_host(ip.into())
    }

    pub fn from_host(ip: IpForHost) -> Self {
        Self {
            ip,
            reached_by: None,
            rtt: None,
            names: vec![],
            service_instances: None,
            last_known_status: LastKnownStatus::Online,
            seen_count: 1,
            last_updated: Instant::now(),
        }
    }

    pub fn with_info(mut self, info: HostUpInfo) -> Self {
        self.reached_by = Some(info.reached_by);
        self.rtt = Some(RttStats::new(info.rtt));
        self
    }

    pub fn with_reached_by(mut self, reached_by: ReachedBy) -> Self {
        self.reached_by = Some(reached_by);
        self
    }

    pub fn with_names(mut self, names: Vec<DnsName>) -> Self {
        self.set_names(names);
        self
    }

    pub fn with_service_instance(mut self, service: ServiceInstance) -> Self {
        self.update_with_service_instance(service);
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

    pub fn ip(&self) -> IpForHost {
        self.ip
    }

    pub fn set_names(&mut self, names: Vec<DnsName>) {
        self.names = names;
    }

    pub fn names(&self) -> &[DnsName] {
        self.names.as_slice()
    }

    pub fn add_name(&mut self, name: DnsName) {
        if !self.names.iter().any(|n| n.matches(&name)) {
            self.names.push(name);
        }
        self.post_process_services();
    }

    pub fn sort_names(&mut self) {
        self.names
            .sort_unstable_by(|a, b| a.canonical().cmp(b.canonical()));
    }

    // Clean the services, removing redudancies and sorting by name
    fn post_process_services(&mut self) {
        self.remove_service_redundancies();
        if let Some(services) = &mut self.service_instances {
            services.sort_unstable_by(|a, b| a.name.cmp(&b.name));
        }
    }

    /// If a service is discovered at some IP but no known hostname exists for that IP, the service
    /// name will appear in the service column, if later we discover a hostname for that IP and it's
    /// the same as the service name, we want to remove the hostname from the service to avoid this
    /// redundancy in the table.
    ///
    /// It might also be misleading as it might show one hostname under the service, and several
    /// hostnames next to the IP, while the service would be reachable under ALL of the hostnames, not
    /// just the specific "original" service name
    fn remove_service_redundancies(&mut self) {
        if let Some(services) = self.service_instances.as_mut() {
            for s in services {
                s.remove_hostname_if_contained_in(&self.names);
            }
        };
    }

    pub fn dedup_names(&mut self) {
        self.names.dedup_by(|a, b| a.matches(b));
    }

    fn names_multiline(&self) -> String {
        let mut names_str = String::new();
        for n in &self.names {
            names_str.push_str(n.display_name());
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
            let unicode_width = name.display_name().width();
            if max < unicode_width {
                max = unicode_width;
            }
        }
        max as u16
    }

    pub fn max_service_instance_unicode_width(&self) -> u16 {
        let mut max = 0;
        for service in self.service_instances.iter().flatten() {
            let unicode_width = service.display_max_line_unicode_width();
            if max < unicode_width {
                max = unicode_width;
            }
        }
        max
    }

    /// Filtering function
    pub fn contains(&self, pattern: &str) -> bool {
        self.ip.to_string().contains(pattern)
            || self
                .names()
                .iter()
                .any(|n| n.display_name().contains(pattern))
            || self.service_instances_multiline().contains(pattern)
    }

    pub fn matches_status(&self, status: LastKnownStatus) -> bool {
        self.last_known_status == status
    }

    pub fn set_last_known_status(
        &mut self,
        (status, new_rtt): (LastKnownStatus, Option<Duration>),
    ) {
        self.last_known_status = status;
        self.set_last_updated_now();
        if let Some(new_rtt) = new_rtt {
            if let Some(cur_rtt) = &mut self.rtt {
                cur_rtt.update(new_rtt);
            } else {
                // This is the case if a DNS-SD service was found at this IP before it was discovered
                // via the network scanner
                self.rtt = Some(RttStats::new(new_rtt))
            }
        }
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
    pub fn update_with_service_instance(&mut self, new_service: ServiceInstance) -> bool {
        for curr_service in self.service_instances.iter_mut().flatten() {
            if curr_service.name == new_service.name && curr_service._type == new_service._type {
                if *curr_service == new_service {
                    return false;
                }
                if let Some(txt) = new_service.txt {
                    if let Some(mut s_txt) = curr_service.txt.take() {
                        for t in txt {
                            if !s_txt.contains(&t) {
                                s_txt.push(t);
                            }
                        }
                        curr_service.txt = Some(s_txt);
                    } else {
                        curr_service.txt = Some(txt);
                    }
                }
                return true;
            }
        }
        if let Some(instances) = &mut self.service_instances {
            instances.push(new_service);
        } else {
            self.service_instances = Some(vec![new_service]);
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

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use super::*;

    fn make_service(name: &str, txt: Option<Vec<String>>) -> ServiceInstance {
        ServiceInstance::new(name.to_owned(), "_http._tcp".to_owned(), None, 80, txt)
    }

    fn make_info() -> IpInfo {
        IpInfo::from_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)))
    }

    /// A service that differs only in txt content is merged into the existing entry.
    #[test]
    fn test_update_with_service_instance_merges_txt() {
        let mut info = make_info();
        info.update_with_service_instance(make_service("web", None));

        // Same service name, different (new) txt data - must merge and return true.
        let updated =
            info.update_with_service_instance(make_service("web", Some(vec!["path=/".to_owned()])));
        assert!(updated);
        let txt = info.services().unwrap()[0].txt.as_ref().unwrap();
        assert!(txt.contains(&"path=/".to_owned()));
    }

    /// Inserting an identical service (same name and all fields) is a no-op.
    #[test]
    fn test_update_with_service_instance_identical_is_noop() {
        let mut info = make_info();
        let svc = make_service("web", Some(vec!["k=v".to_owned()]));
        info.update_with_service_instance(svc.clone());

        let updated = info.update_with_service_instance(svc);
        assert!(!updated);
        assert_eq!(info.services().unwrap().len(), 1);
    }

    /// Merging a reverse-DNS hostname (no trailing dot) with the same hostname
    /// as an absolute FQDN (trailing dot) must produce exactly one name.
    #[test]
    fn test_merge_deduplicates_trailing_dot_hostnames() {
        let mut base = make_info();
        base.set_names(vec![DnsName::new("hostname.local")]);

        let mut other = make_info();
        other.set_names(vec![DnsName::new("hostname.local.")]);

        base.merge(other);

        let names: Vec<&str> = base.names().iter().map(DnsName::display_name).collect();
        assert_eq!(
            names,
            ["hostname.local"],
            "expected exactly one name after merging with/without trailing dot"
        );
    }

    /// Inverse order: absolute FQDN arrives first, unqualified form via merge.
    #[test]
    fn test_merge_deduplicates_trailing_dot_hostnames_inverse_order() {
        let mut base = make_info();
        base.set_names(vec![DnsName::new("hostname.local.")]);

        let mut other = make_info();
        other.set_names(vec![DnsName::new("hostname.local")]);

        base.merge(other);

        let names: Vec<&str> = base.names().iter().map(DnsName::display_name).collect();
        assert_eq!(
            names,
            ["hostname.local"],
            "expected exactly one name after merging with/without trailing dot (inverse order)"
        );
    }

    /// Adding an absolute FQDN (trailing dot) for an already-known hostname
    /// must not create a duplicate entry.
    #[test]
    fn test_add_name_deduplicates_trailing_dot_hostnames() {
        let mut info = make_info();
        info.set_names(vec![DnsName::new("hostname.local")]);
        info.add_name(DnsName::new("hostname.local."));

        let names: Vec<&str> = info.names().iter().map(DnsName::display_name).collect();
        assert_eq!(
            names,
            ["hostname.local"],
            "expected exactly one name after add_name with trailing dot"
        );
    }

    /// Inverse order: absolute FQDN stored first, unqualified form added later.
    #[test]
    fn test_add_name_deduplicates_trailing_dot_hostnames_inverse_order() {
        let mut info = make_info();
        info.set_names(vec![DnsName::new("hostname.local.")]);
        info.add_name(DnsName::new("hostname.local"));

        let names: Vec<&str> = info.names().iter().map(DnsName::display_name).collect();
        assert_eq!(
            names,
            ["hostname.local"],
            "expected exactly one name after add_name without trailing dot (inverse order)"
        );
    }

    const PRINTER_NAME: &str = "My Printer";
    const HTTP_TYPE: &str = "_http._tcp";
    const PRINTER_TYPE: &str = "_printer._tcp";
    const HTTP_PORT: u16 = 80;
    const PRINTER_PORT: u16 = 515;

    /// A second service with a different name is appended, not merged.
    #[test]
    fn test_update_with_service_instance_appends_new_name() {
        let mut info = make_info();
        info.update_with_service_instance(make_service("web", None));
        info.update_with_service_instance(make_service("api", None));
        assert_eq!(info.services().unwrap().len(), 2);
    }

    /// DIFFERENT service types with the SAME instance name must NOT be merged.
    #[test]
    fn test_service_merging_same_name_different_type() {
        let mut info = make_info();

        let svc1 = ServiceInstance::new(
            PRINTER_NAME.to_owned(),
            HTTP_TYPE.to_owned(),
            None,
            HTTP_PORT,
            None,
        );
        let svc2 = ServiceInstance::new(
            PRINTER_NAME.to_owned(),
            PRINTER_TYPE.to_owned(),
            None,
            PRINTER_PORT,
            None,
        );

        info.update_with_service_instance(svc1);
        info.update_with_service_instance(svc2);

        let services = info.services().unwrap();
        assert_eq!(services.len(), 2);
    }
}
