use mds_ipinfo::IpForHost;

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
    pub ipv4: Option<Ipv4Addr>,
    pub ipv6: Option<Ipv6Addr>,
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

        let ips: Vec<IpAddr> = self
            .ips_hostnames
            .get_ips_by_hostname(&normalized_host)
            .into_iter()
            .copied()
            .collect();

        let info = self.get_or_create(instance);
        debug_assert!(
            info.host.is_none()
                || normalize_hostname(info.host.as_ref().unwrap()) == normalized_host,
            "mismatch: current host: {:?}, new host: {hostname}",
            info.host
        );
        debug_assert!(
            info.port.is_none() || info.port == Some(port),
            "mismatch: current port: {:?}, new port: {port}",
            info.port
        );
        info.host = Some(hostname);
        info.port = Some(port);

        for ip in ips {
            match ip {
                IpAddr::V4(ipv4) => {
                    debug_assert!(info.ipv4.is_none() || info.ipv4 == Some(ipv4));
                    info.ipv4 = Some(ipv4)
                }
                IpAddr::V6(ipv6) => {
                    debug_assert!(info.ipv6.is_none() || info.ipv6 == Some(ipv6));
                    info.ipv6 = Some(ipv6)
                }
            }
        }
    }

    pub(crate) fn set_ip_for_host(&mut self, hostname: &str, ip: IpAddr) {
        let normalized_host = normalize_hostname(hostname);
        self.ips_hostnames.insert(ip, normalized_host.clone());

        for info in self.services.values_mut() {
            if let Some(ref host) = info.host
                && normalize_hostname(host) == normalized_host {
                    match ip {
                        IpAddr::V4(ipv4) => {
                            debug_assert!(info.ipv4.is_none() || info.ipv4 == Some(ipv4));
                            info.ipv4 = Some(ipv4);
                        }
                        IpAddr::V6(ipv6) => {
                            debug_assert!(info.ipv6.is_none() || info.ipv6 == Some(ipv6));
                            info.ipv6 = Some(ipv6);
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
            let ip = match IpForHost::try_from((*ipv4, *ipv6)) {
                Ok(ip) => ip,
                Err(e) => {
                    log::debug!("Dropping partially resolved service: {e}");
                    continue;
                }
            };

            // Trim the service type suffix from name
            let name = name
                .strip_suffix(_type.as_str()) // strip in two steps to avoid allocations
                .and_then(|s| s.strip_suffix('.'))
                .unwrap_or(name)
                .to_string();
            final_services.push(ServiceInfo {
                name,
                _type: _type.clone(),
                txt: txt.clone(),
                host: host.clone(),
                ip,
                port: *port,
            });
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

fn normalize_hostname(host: &str) -> String {
    host.strip_suffix('.').unwrap_or(host).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SERVICE_INSTANCE: &str = "MyService._http._tcp.local.";
    const TEST_SERVICE_TYPE: &str = "_http._tcp.local.";
    const TEST_HOSTNAME_ABSOLUTE: &str = "myhost.local.";
    const TEST_HOSTNAME_RELATIVE: &str = "myhost.local";
    const TEST_IPV4: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 100);
    const TEST_PORT: u16 = 80;

    #[test]
    fn test_hostname_normalization_missing_trailing_dot() {
        let mut registry = ServiceRegistry::default();

        registry.insert_or_update_instance(TEST_SERVICE_INSTANCE, TEST_SERVICE_TYPE.to_string());
        registry.set_srv(
            TEST_SERVICE_INSTANCE,
            TEST_HOSTNAME_ABSOLUTE.to_string(),
            TEST_PORT,
        );

        // A record for the host WITHOUT a trailing dot
        registry.set_ip_for_host(TEST_HOSTNAME_RELATIVE, IpAddr::V4(TEST_IPV4));

        let results = registry.finalize();
        assert_eq!(results.len(), 1);
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
        assert_eq!(results.len(), 1);
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

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].ip, IpForHost::V4(TEST_IPV4));
    }
}
