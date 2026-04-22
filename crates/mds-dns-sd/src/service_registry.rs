use mds_ipinfo::IpForHost;
use mds_util::prelude::normalize_hostname;
use smallvec::SmallVec;

use crate::bivec::IpHostnameLookupVec;

use super::ServiceInfo;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Debug, Default)]
pub struct TempServiceInfo {
    pub name: String,
    pub _type: Option<String>,
    pub txt: Option<Vec<String>>,
    pub host: Option<String>,
    pub ipv4: SmallVec<[Ipv4Addr; 2]>,
    pub ipv6: SmallVec<[Ipv6Addr; 2]>,
    pub port: Option<u16>,
}

#[derive(Debug, Default)]
pub struct ServiceRegistry {
    services: HashMap<String, TempServiceInfo>,
    ips_hostnames: IpHostnameLookupVec,
    cname_aliases: HashMap<String, String>,
    mail_exchanges: HashMap<String, Vec<(String, u16)>>, // domain -> [(server, priority)]
    nameservers: HashMap<String, Vec<String>>,           // domain -> [servers]
    soa_records: HashMap<String, (String, String, u32)>, // domain -> (primary_ns, admin_email, serial)
}

impl ServiceRegistry {
    pub(crate) fn insert_or_update_instance(
        &mut self,
        instance: impl AsRef<str>,
        service_type: String,
    ) {
        self.get_or_create(instance.as_ref())
            ._type
            .get_or_insert(service_type);
    }

    pub(crate) fn set_txt(&mut self, instance: &str, txt: Vec<String>) {
        self.get_or_create(instance).txt = Some(txt);
    }

    pub(crate) fn set_srv(&mut self, instance: &str, hostname: String, port: u16) {
        let normalized_host = normalize_hostname(&hostname);

        // A/AAAA records may arrive before the SRV record that references them,
        // so pick up any addresses already recorded for this host.
        let ips: Vec<IpAddr> = self
            .ips_hostnames
            .get_ips_by_hostname(&normalized_host)
            .into_iter()
            .copied()
            .collect();

        // Conflicting SRV answers for the same instance within one collection
        // window are abnormal (RFC 6762 name conflict or a stale record)
        let info = self.get_or_create(instance);
        if let Some(prev) = &info.host
            && normalize_hostname(prev) != normalized_host
        {
            log::warn!("SRV target for '{instance}' changed from '{prev}' to '{hostname}'");
        }
        if let Some(prev_port) = info.port
            && prev_port != port
        {
            log::warn!("SRV port for '{instance}' changed from {prev_port} to {port}");
        }
        // Keep the advertised spelling for display (matching uses the normalized form)
        info.host = Some(hostname);
        info.port = Some(port);

        for ip in ips {
            match ip {
                IpAddr::V4(v4) => {
                    if !info.ipv4.contains(&v4) {
                        info.ipv4.push(v4);
                    }
                }
                IpAddr::V6(v6) => {
                    if !info.ipv6.contains(&v6) {
                        info.ipv6.push(v6);
                    }
                }
            }
        }
    }

    pub(crate) fn set_ip_for_host(&mut self, hostname: &str, ip: IpAddr) {
        let normalized_host = normalize_hostname(hostname);
        self.ips_hostnames.insert(ip, normalized_host.clone());

        for info in self.services.values_mut() {
            if let Some(ref host) = info.host
                && normalize_hostname(host) == normalized_host
            {
                match ip {
                    IpAddr::V4(v4) => {
                        if !info.ipv4.contains(&v4) {
                            info.ipv4.push(v4);
                        }
                    }
                    IpAddr::V6(v6) => {
                        if !info.ipv6.contains(&v6) {
                            info.ipv6.push(v6);
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn set_cname_alias(&mut self, hostname: &str, canonical: String) {
        self.cname_aliases.insert(hostname.to_string(), canonical);
    }

    pub(crate) fn set_mail_exchange(&mut self, domain: &str, server: String, priority: u16) {
        self.mail_exchanges
            .entry(domain.to_string())
            .or_default()
            .push((server, priority));
    }

    pub(crate) fn set_nameserver(&mut self, domain: &str, server: String) {
        self.nameservers
            .entry(domain.to_string())
            .or_default()
            .push(server);
    }

    pub(crate) fn set_soa(
        &mut self,
        domain: &str,
        primary_ns: String,
        admin_email: String,
        serial: u32,
    ) {
        self.soa_records
            .insert(domain.to_string(), (primary_ns, admin_email, serial));
    }

    fn get_or_create(&mut self, instance: &str) -> &mut TempServiceInfo {
        self.services
            .entry(instance.to_owned())
            .or_insert_with(|| TempServiceInfo {
                name: instance.to_owned(),
                ..Default::default()
            })
    }

    /// From all the collected [`TempServiceInfo`], filter out partially resolved services and return all the complete (enough) ones as [`ServiceInfo`]
    pub(crate) fn finalize(&self) -> Vec<ServiceInfo> {
        let mut final_services = Vec::with_capacity(self.services.len());
        for TempServiceInfo {
            name,
            _type,
            txt,
            host,
            ipv4,
            ipv6,
            port,
        } in self.services.values()
        {
            let Some(host) = host else {
                log::debug!("Dropping partially resolved service: Missing hostname");
                continue;
            };
            let Some(port) = port else {
                log::debug!("Dropping partially resolved service: Missing port");
                continue;
            };
            let Some(_type) = _type else {
                log::debug!("Dropping partially resolved service: Missing service type");
                continue;
            };
            if ipv4.is_empty() && ipv6.is_empty() {
                log::debug!("Dropping partially resolved service: Missing IP address");
                continue;
            }

            // A host can announce several addresses. Pair an IPv4 with an IPv6 while
            // both are available: the collector merges table rows that share an IP, so
            // a paired entry links a dual-stack host's IPv4 and IPv6 rows into one.
            // Leftover addresses are emitted individually so every address on a
            // multi-homed host still gets the service.
            let mut ips = Vec::with_capacity(ipv4.len().max(ipv6.len()));
            let mut v4s = ipv4.iter().copied();
            let mut v6s = ipv6.iter().copied();
            loop {
                match (v4s.next(), v6s.next()) {
                    (Some(v4), Some(v6)) => ips.push(IpForHost::V4andV6((v4, v6))),
                    (Some(v4), None) => ips.push(IpForHost::V4(v4)),
                    (None, Some(v6)) => ips.push(IpForHost::V6(v6)),
                    (None, None) => break,
                }
            }

            // Trim the service type suffix from name
            let name = name
                .strip_suffix(_type.as_str()) // strip in two steps to avoid allocations
                .and_then(|s| s.strip_suffix('.'))
                .unwrap_or(name)
                .to_string();

            for ip in ips {
                final_services.push(ServiceInfo {
                    name: name.clone(),
                    _type: _type.clone(),
                    txt: txt.clone(),
                    host: host.clone(),
                    ip,
                    port: *port,
                });
            }
        }
        final_services
    }

    pub fn cname_aliases(&self) -> &HashMap<String, String> {
        &self.cname_aliases
    }

    pub fn mail_exchanges(&self) -> &HashMap<String, Vec<(String, u16)>> {
        &self.mail_exchanges
    }

    pub fn nameservers(&self) -> &HashMap<String, Vec<String>> {
        &self.nameservers
    }

    pub fn soa_records(&self) -> &HashMap<String, (String, String, u32)> {
        &self.soa_records
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SERVICE_INSTANCE: &str = "MyService._http._tcp.local.";
    const TEST_SERVICE_TYPE: &str = "_http._tcp.local.";
    const TEST_HOSTNAME_ABSOLUTE: &str = "myhost.local.";
    const TEST_HOSTNAME_RELATIVE: &str = "myhost.local";
    const TEST_HOSTNAME_MIXED_CASE: &str = "MyHost.Local.";
    const TEST_IPV4: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 100);
    const TEST_IPV4_2: Ipv4Addr = Ipv4Addr::new(10, 0, 0, 1);
    const TEST_IPV6: Ipv6Addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
    const TEST_PORT: u16 = 80;

    fn registry_with_srv() -> ServiceRegistry {
        let mut registry = ServiceRegistry::default();
        registry.insert_or_update_instance(TEST_SERVICE_INSTANCE, TEST_SERVICE_TYPE.to_string());
        registry.set_srv(
            TEST_SERVICE_INSTANCE,
            TEST_HOSTNAME_ABSOLUTE.to_string(),
            TEST_PORT,
        );
        registry
    }

    #[test]
    fn test_hostname_normalization_missing_trailing_dot() {
        let mut registry = registry_with_srv();

        // A record for the host WITHOUT a trailing dot
        registry.set_ip_for_host(TEST_HOSTNAME_RELATIVE, IpAddr::V4(TEST_IPV4));

        let results = registry.finalize();
        let ips: Vec<IpForHost> = results.iter().map(|s| s.ip).collect();
        assert_eq!(ips, [IpForHost::V4(TEST_IPV4)]);
    }

    #[test]
    fn test_hostname_normalization_extra_trailing_dot() {
        let mut registry = ServiceRegistry::default();

        registry.insert_or_update_instance(TEST_SERVICE_INSTANCE, TEST_SERVICE_TYPE.to_string());
        // SRV record WITHOUT a trailing dot
        registry.set_srv(
            TEST_SERVICE_INSTANCE,
            TEST_HOSTNAME_RELATIVE.to_string(),
            TEST_PORT,
        );

        // A record WITH a trailing dot
        registry.set_ip_for_host(TEST_HOSTNAME_ABSOLUTE, IpAddr::V4(TEST_IPV4));

        let results = registry.finalize();
        let ips: Vec<IpForHost> = results.iter().map(|s| s.ip).collect();
        assert_eq!(ips, [IpForHost::V4(TEST_IPV4)]);
    }

    /// RFC 6762 section 16: mDNS names differing only in ASCII case are the same name
    #[test]
    fn test_hostname_case_insensitive_matching() {
        let mut registry = registry_with_srv();

        // A record with different casing than the SRV target
        registry.set_ip_for_host(TEST_HOSTNAME_MIXED_CASE, IpAddr::V4(TEST_IPV4));

        let results = registry.finalize();
        let ips: Vec<IpForHost> = results.iter().map(|s| s.ip).collect();
        assert_eq!(ips, [IpForHost::V4(TEST_IPV4)]);
        // The SRV spelling is preserved for display
        assert_eq!(results[0].host, TEST_HOSTNAME_ABSOLUTE);
    }

    #[test]
    fn test_out_of_order_resolution() {
        let mut registry = ServiceRegistry::default();

        // A record arrives FIRST
        registry.set_ip_for_host(TEST_HOSTNAME_ABSOLUTE, IpAddr::V4(TEST_IPV4));
        // SRV record arrives LATER
        registry.insert_or_update_instance(TEST_SERVICE_INSTANCE, TEST_SERVICE_TYPE.to_string());
        registry.set_srv(
            TEST_SERVICE_INSTANCE,
            TEST_HOSTNAME_ABSOLUTE.to_string(),
            TEST_PORT,
        );

        let results = registry.finalize();

        let ips: Vec<IpForHost> = results.iter().map(|s| s.ip).collect();
        assert_eq!(ips, [IpForHost::V4(TEST_IPV4)]);
    }

    /// A host announcing several addresses of the same family gets one entry per address
    #[test]
    fn test_multi_homed_host_resolution() {
        let mut registry = registry_with_srv();

        // Two different A records for the same host
        registry.set_ip_for_host(TEST_HOSTNAME_ABSOLUTE, IpAddr::V4(TEST_IPV4));
        registry.set_ip_for_host(TEST_HOSTNAME_ABSOLUTE, IpAddr::V4(TEST_IPV4_2));

        let results = registry.finalize();

        let ips: Vec<IpForHost> = results.iter().map(|s| s.ip).collect();
        assert_eq!(ips, [IpForHost::V4(TEST_IPV4), IpForHost::V4(TEST_IPV4_2)]);
    }

    /// A dual-stack host's IPv4 and IPv6 are paired into a single entry so the
    /// collector can merge the host's rows
    #[test]
    fn test_dual_stack_host_pairs_addresses() {
        let mut registry = registry_with_srv();

        registry.set_ip_for_host(TEST_HOSTNAME_ABSOLUTE, IpAddr::V4(TEST_IPV4));
        registry.set_ip_for_host(TEST_HOSTNAME_ABSOLUTE, IpAddr::V6(TEST_IPV6));

        let results = registry.finalize();

        let ips: Vec<IpForHost> = results.iter().map(|s| s.ip).collect();
        assert_eq!(ips, [IpForHost::V4andV6((TEST_IPV4, TEST_IPV6))]);
    }

    /// Duplicate A records (e.g. repeated announcements) must not create duplicate entries
    #[test]
    fn test_repeated_a_record_is_deduplicated() {
        let mut registry = registry_with_srv();

        registry.set_ip_for_host(TEST_HOSTNAME_ABSOLUTE, IpAddr::V4(TEST_IPV4));
        registry.set_ip_for_host(TEST_HOSTNAME_ABSOLUTE, IpAddr::V4(TEST_IPV4));

        let results = registry.finalize();
        let ips: Vec<IpForHost> = results.iter().map(|s| s.ip).collect();
        assert_eq!(ips, [IpForHost::V4(TEST_IPV4)]);
    }
}
