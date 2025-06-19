use crate::AppConfig;

impl PartialEq for AppConfig {
    fn eq(&self, other: &Self) -> bool {
        // Destructure 'self' to explicitly list and compare all relevant fields.
        // This way adding a new field will cause a compilation error, so we won't accidentally
        // not add it to this implementation
        let AppConfig {
            iface_ignore_re,
            iface_include_docker,
            service_discovery,
            compact,
            tcp_port_timeout_ms,
            ping_timeout_ms,
            ip_check_timeout_ms,
            compiled_iface_ignore_re: _, // This field is intentionally skipped in comparison
        } = self;

        // Destructure 'other' similarly.
        let AppConfig {
            iface_ignore_re: other_iface_ignore_re,
            iface_include_docker: other_iface_include_docker,
            service_discovery: other_service_discovery,
            compact: other_compact,
            tcp_port_timeout_ms: other_tcp_port_timeout_ms,
            ping_timeout_ms: other_ping_timeout_ms,
            ip_check_timeout_ms: other_ip_check_timeout_ms,
            compiled_iface_ignore_re: _, // This field is intentionally skipped in comparison
        } = other;

        // Now compare each field explicitly. If a new field is added to AppConfig
        // and not included in the destructuring pattern above, the compiler will warn/error.
        iface_ignore_re == other_iface_ignore_re
            && iface_include_docker == other_iface_include_docker
            && service_discovery == other_service_discovery
            && compact == other_compact
            && tcp_port_timeout_ms == other_tcp_port_timeout_ms
            && ping_timeout_ms == other_ping_timeout_ms
            && ip_check_timeout_ms == other_ip_check_timeout_ms
    }
}
