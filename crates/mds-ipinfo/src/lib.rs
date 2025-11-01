use std::{
    fmt::{self, Display},
    net::IpAddr,
    time::{Duration, Instant},
};

use mds_util::host_up::{HostUpInfo, ReachedBy};
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IpInfo {
    pub ip: IpForHost,
    pub reached_by: Option<ReachedBy>,
    /// RTT on the first time the host was detected
    pub rtt: Option<RttStats>,
    pub names: Vec<String>,
    pub service_instances: Option<Vec<ServiceInstance>>,
    pub last_known_status: LastKnownStatus,
    pub seen_count: u64,
    pub last_updated: Instant,
}

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
        self.names.sort_unstable();
        self.names.dedup();

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
            self.names_multiline().clone(),
            self.seen_count.to_string(),
            self.service_instances_multiline(),
        ]
    }

    pub fn from_ip(ip: IpAddr) -> Self {
        Self {
            ip: ip.into(),
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

    pub fn set_names(&mut self, names: Vec<String>) {
        self.names = names
    }

    pub fn names(&self) -> &[String] {
        self.names.as_slice()
    }

    pub fn add_name(&mut self, name: String) {
        self.names.push(name);
        self.post_process_services();
    }

    pub fn sort_names(&mut self) {
        self.names.sort();
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
            || self.names().iter().any(|n| n.contains(pattern))
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
            if curr_service.name == new_service.name {
                if *curr_service == new_service {
                    return false;
                } else {
                    if cfg!(debug_assertions) {
                        let curr_service_name = &curr_service.hostname;
                        let curr_service_type = &curr_service._type;
                        let curr_service_port = curr_service.port;

                        let new_service_name = &new_service.hostname;
                        let new_service_type = &new_service._type;
                        let new_service_port = new_service.port;
                        let type_eq = curr_service_type == new_service_type;
                        // The new service hostname is allowed to be `None` as it is set to `None` in the case where it advertises under the
                        // same hostname as an already known host
                        let name_eq =
                            curr_service_name == new_service_name || new_service_name.is_none();
                        let port_eq = curr_service_port == new_service_port;
                        assert!(
                            (type_eq && name_eq && port_eq),
                            "Mismatch between existing service and new service to update it with:\
                                \nExisting service vs. New service\
                                \nType:     {curr_service_type} | {new_service_type}\
                                \nPort:     {curr_service_port} | {new_service_port}\
                                \nHostname: {curr_service_name:?} | {new_service_name:?}\
                                \n--- Full Services ---\
                                \nExisting:\
                                \n{curr_service:?}\
                                \nNew:\
                                \n{new_service:?}"
                        );
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
