use crate::AppConfig;

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            iface_ignore_re: mds_default::IFACE_IGNORE_RE
                .value
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            iface_include_docker: mds_default::IFACE_INCLUDE_DOCKER.value,
            service_discovery: mds_default::SERVICE_DISCOVERY.value,
            compact: mds_default::COMPACT.value,
            tcp_port_timeout_ms: mds_default::TCP_PORT_TIMEOUT_MS.value,
            ping_timeout_ms: mds_default::PING_TIMEOUT_MS.value,
            ip_check_timeout_ms: mds_default::IP_CHECK_TIMEOUT_MS.value,
            compiled_iface_ignore_re: None,
        }
    }
}
