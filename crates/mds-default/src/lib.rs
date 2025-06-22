mod default;

const DEFAULT_CONFIG: &str = include_str!("../../../docs/default_config.toml");

pub fn default_config_without_doc_header() -> String {
    let mut default_config = String::new();
    let mut start_including = false;
    for l in DEFAULT_CONFIG.lines() {
        if !start_including && l.starts_with("#") {
            // ignore lines until the first non-commented lines
            // by convention, the default header ends with an empty non-commented line
        } else if start_including {
            default_config.push_str(l);
            default_config.push('\n');
        } else {
            start_including = true;
        }
    }
    default_config.push('\n');
    default_config
}

config_fields! {
    /// Enable DNS service discovery (DNS-SD)
    /// When enabled, attempts to discover services advertised via DNS-SD/mDNS
    service_discovery: bool = true;

    #[section]
    Ui {
        /// Use compact output format
        /// Hides help footer
        compact: bool = false;

        /// Hide IPs with no association (no resolved hostname/service information)
        hide_bare_ips: bool = false;
    }


    #[section]
    Interfaces {
        /// Regex patterns to ignore network interfaces
        /// These patterns will be matched against interface names to exclude them from scanning
        /// NOTE: Common virtual interfaces are already excluded through the 'include_docker' setting
        ignore_patterns: &[&str] = &[];

        /// Include docker network interfaces in scans
        /// By default, docker interfaces are excluded for cleaner results.
        /// Any interface with the following prefixes are excluded
        /// - veth
        /// - br-
        /// - podman
        /// - docker
        include_docker: bool = false;
    }

    #[section]
    Timeouts {
        /// How long to wait for TCP connection attempts
        /// This timeout applies to each port individually
        /// Range: 1-6535
        tcp_port_ms: u16 = 100;
        /// How long to wait for ping/echo replies
        /// Range: 1-6535
        ping_ms: u16 = 300;

        /// Upper time limit for checking if a host is up on an IP address
        /// This is the total timeout for all host-up detection methods combined
        /// Range: 1-6535
        ip_check_ms: u16 = 5000;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_str_eq;

    #[test]
    fn test_access_description() {
        let compact_desc = UI_COMPACT.description;
        println!("{compact_desc}");
    }

    #[test]
    fn test_gen_default_config_matches() {
        let final_str = generate_default_toml_sections();
        println!("---");
        print!("{final_str}");
        println!("---");
        assert_str_eq!(default_config_without_doc_header(), final_str);
    }
}
