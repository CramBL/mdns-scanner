use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServiceInstance {
    pub(crate) name: String,
    // Only applicable if it advertises an mDNS hostname by itself that doesn't match the hostname of the host at the IP its at
    pub(crate) hostname: Option<String>,
    pub(crate) _type: String,
    pub(crate) port: u16,
    pub(crate) txt: Option<Vec<String>>,
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

    pub fn remove_hostname_if_contained_in(&mut self, names: &[String]) {
        let _ = self.hostname.take_if(|h| names.contains(h));
    }
}

impl fmt::Display for ServiceInstance {
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
