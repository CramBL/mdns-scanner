mod default;

config_fields! {
    iface_ignore_re: &[&str] = &[];
    iface_include_docker: bool = false;
    service_discovery: bool = true;
    compact: bool = false;
    tcp_port_timeout_ms: u16 = 100;
    ping_timeout_ms: u16 = 300;
    ip_check_timeout_ms: u16 = 5000;
    hide_bare_ips: bool = false;
}
