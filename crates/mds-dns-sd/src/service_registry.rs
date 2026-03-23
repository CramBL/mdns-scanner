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
        let info = self.get_or_create(instance);
        debug_assert!(
            info.host.is_none() || info.host == Some(hostname.clone()),
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
    }

    pub(crate) fn set_ip_for_host(&mut self, hostname: &str, ip: IpAddr) {
        self.ips_hostnames.insert(ip, hostname.to_owned());
        for info in self.services.values_mut() {
            if let Some(ref host) = info.host
                && host == hostname
            {
                for ip in self.ips_hostnames.get_ips_by_hostname(hostname) {
                    match ip {
                        IpAddr::V4(ipv4) => {
                            debug_assert!(info.ipv4.is_none() || info.ipv4 == Some(*ipv4));
                            info.ipv4 = Some(*ipv4)
                        }
                        IpAddr::V6(ipv6) => {
                            debug_assert!(info.ipv6.is_none() || info.ipv6 == Some(*ipv6));
                            info.ipv6 = Some(*ipv6)
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
            debug_assert!(
                ipv4.is_some() || ipv6.is_some(),
                "There should always be either an Ipv4 or an Ipv6. Failed for service: name={name:?}"
            );
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
