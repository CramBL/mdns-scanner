use std::collections::HashMap;
use std::net::IpAddr;

use super::ServiceInfo;

#[derive(Debug, Default)]
pub struct TempServiceInfo {
    pub name: String,
    pub _type: String,
    pub txt: Option<Vec<String>>,
    pub host: Option<String>,
    pub ip: Option<IpAddr>,
    pub port: Option<u16>,
}

pub struct ServiceRegistry {
    services: HashMap<String, TempServiceInfo>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    pub fn insert_or_update_instance(&mut self, instance: String, service_type: String) {
        self.services
            .entry(instance.clone())
            .or_insert_with(|| TempServiceInfo {
                name: instance,
                _type: service_type,
                ..Default::default()
            });
    }

    pub fn set_txt(&mut self, instance: &str, txt: Vec<String>) {
        if let Some(info) = self.services.get_mut(instance) {
            info.txt = Some(txt);
        }
    }

    pub fn set_srv(&mut self, instance: &str, hostname: String, port: u16) {
        if let Some(info) = self.services.get_mut(instance) {
            info.host = Some(hostname);
            info.port = Some(port);
        }
    }

    pub fn set_ip_for_host(&mut self, hostname: &str, ip: IpAddr) {
        for info in self.services.values_mut() {
            if let Some(ref host) = info.host {
                if host == hostname {
                    info.ip = Some(ip);
                }
            }
        }
    }

    pub fn finalize(&self) -> Vec<ServiceInfo> {
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
}
