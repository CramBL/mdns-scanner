use mds_default::default_config_without_doc_header;

use crate::{AppConfig, timeouts::Timeouts};

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
            timeouts: Timeouts {
                tcp_port_ms: mds_default::TIMEOUTS_TCP_PORT_MS.value.try_into().unwrap(),
                ping_ms: mds_default::TIMEOUTS_PING_MS.value.try_into().unwrap(),
                ip_check_ms: mds_default::TIMEOUTS_IP_CHECK_MS.value.try_into().unwrap(),
            },
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

        let def_conf = default_config_without_doc_header();
        config.push_str(&def_conf);
        config
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;

    #[test]
    fn test_gen_default_config_str() -> TestResult {
        let default = AppConfig::default_config();
        assert!(default.starts_with("#"));

        let config: AppConfig = toml::from_str(&default)?;

        println!("{config:?}");

        assert_eq!(config, AppConfig::default());

        Ok(())
    }
}
