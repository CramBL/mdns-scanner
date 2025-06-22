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
            hide_bare_ips: mds_default::HIDE_BARE_IPS.value,
            compiled_iface_ignore_re: None,
        }
    }
}

const DEFAULT_CONFIG: &str = include_str!("../../../docs/default_config.toml");
const DEFAULT_CONFIG_HEADER: &str = "\
# mdns-scanner configuration file
#
# Possible Locations:
";
const CONFIG_LOC_DESCRIPTION: &str = if cfg!(target_os = "windows") {
    r"
# - %APPDATA%\mdns-scanner\config.toml    (user-level, persisted)
# - .\mdns-scanner.toml                   (directory local)"
} else if cfg!(target_os = "macos") {
    "\
# - ~/Library/Application Support/mdns-scanner/config.toml    (user-level, persisted)
# - ./mdns-scanner.toml                                       (directory local)"
} else {
    "\
# - ~/.config/mdns-scanner/config.toml    (user-level, persisted)
# - ./mdns-scanner.toml                   (directory local)"
};

impl AppConfig {
    /// Return the default config as a string
    pub fn default_config() -> String {
        let mut config = String::with_capacity(
            DEFAULT_CONFIG.len() + DEFAULT_CONFIG_HEADER.len() + CONFIG_LOC_DESCRIPTION.len(),
        );

        config.push_str(DEFAULT_CONFIG_HEADER);
        config.push_str(CONFIG_LOC_DESCRIPTION);
        config.push('\n');
        config.push('\n');

        let mut start_including = false;
        for l in DEFAULT_CONFIG.lines() {
            if !start_including && l.starts_with("#") {
                // ignore lines until the first non-commented lines
                // by convention, the default header ends with an empty non-commented line
            } else if start_including {
                config.push_str(l);
                config.push('\n');
            } else {
                start_including = true;
            }
        }
        config.push('\n');
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_default_config_str() {
        let default = AppConfig::default_config();
        assert!(default.starts_with("#"));
    }
}
