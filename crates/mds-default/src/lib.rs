mod default;

config_fields! {
    iface_ignore_re: &[&str] = &[] => "\
# Regex patterns to ignore network interfaces
# These patterns will be matched against interface names to exclude them from scanning
# NOTE: Common virtual interfaces are already excluded through the 'iface_include_docker' setting";
    iface_include_docker: bool = false => "\
# Include docker network interfaces in scans
# By default, docker interfaces are excluded for cleaner results.
# Any interface with the following prefixes are excluded
# - veth
# - br-
# - podman
# - docker";
    service_discovery: bool = true => "\
# Enable DNS service discovery (DNS-SD)
# When enabled, attempts to discover services advertised via DNS-SD/mDNS
";
    compact: bool = false => "\
# Use compact output format
# Hides help footer";
    tcp_port_timeout_ms: u16 = 100 => "";
    ping_timeout_ms: u16 = 300 => "";
    ip_check_timeout_ms: u16 = 5000 => "";
    hide_bare_ips: bool = false => "";
}
