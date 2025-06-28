use super::ServiceInfo;
use std::collections::HashMap;
use std::net::IpAddr;

#[derive(Debug, Default)]
pub struct TempServiceInfo {
    pub name: String,
    pub _type: String,
    pub txt: Option<Vec<String>>,
    pub host: Option<String>,
    pub ip: Option<IpAddr>,
    pub port: Option<u16>,
}

#[derive(Debug, Default)]
pub struct ServiceRegistry {
    services: HashMap<String, TempServiceInfo>,

    cname_aliases: HashMap<String, String>,
    mail_exchanges: HashMap<String, Vec<(String, u16)>>, // domain -> [(server, priority)]
    nameservers: HashMap<String, Vec<String>>,           // domain -> [servers]
    soa_records: HashMap<String, (String, String, u32)>, // domain -> (primary_ns, admin_email, serial)
}

impl ServiceRegistry {
    pub(crate) fn insert_or_update_instance(&mut self, instance: String, service_type: String) {
        self.services
            .entry(instance.clone())
            .or_insert_with(|| TempServiceInfo {
                name: instance,
                _type: service_type,
                ..Default::default()
            });
    }

    pub(crate) fn set_txt(&mut self, instance: &str, txt: Vec<String>) {
        if let Some(info) = self.services.get_mut(instance) {
            info.txt = Some(txt);
        }
    }

    pub(crate) fn set_srv(&mut self, instance: &str, hostname: String, port: u16) {
        if let Some(info) = self.services.get_mut(instance) {
            info.host = Some(hostname);
            info.port = Some(port);
        }
    }

    pub(crate) fn set_ip_for_host(&mut self, hostname: &str, ip: IpAddr) {
        for info in self.services.values_mut() {
            if let Some(ref host) = info.host {
                if host == hostname {
                    info.ip = Some(ip);
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

    pub(crate) fn finalize(&self) -> Vec<ServiceInfo> {
        self.services
            .values()
            .filter_map(|temp| {
                let host = temp.host.clone()?;
                let ip = temp.ip?;
                let port = temp.port?;
                // Trim the service type suffix from name
                let name = temp
                    .name
                    .strip_suffix(&format!(".{}", temp._type))
                    .unwrap_or(&temp.name)
                    .to_string();
                Some(ServiceInfo {
                    name,
                    _type: temp._type.clone(),
                    txt: temp.txt.clone(),
                    host,
                    ip,
                    port,
                })
            })
            .collect()
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
