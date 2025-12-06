mod default;

pub const DEFAULT_CONFIG: &str = include_str!("../../../docs/default_config.toml");
pub const DEFAULT_KEYMAP: &str = include_str!("../../../docs/default_keymap.toml");

pub fn default_config_without_doc_header() -> String {
    default_file_without_doc_header(DEFAULT_CONFIG)
}

pub fn default_keymap_without_doc_header() -> String {
    default_file_without_doc_header(DEFAULT_KEYMAP)
}

fn default_file_without_doc_header(contents: &str) -> String {
    let mut default_file = String::new();
    let mut start_including = false;
    for l in contents.lines() {
        if !start_including && l.starts_with("#") {
            // ignore lines until the first non-commented lines
            // by convention, the default header ends with an empty non-commented line
        } else if start_including {
            default_file.push_str(l);
            default_file.push('\n');
        } else {
            start_including = true;
        }
    }
    default_file.push('\n');
    default_file
}

config_fields! {
    #[section]
    Scan {
        /// Enable DNS service discovery (DNS-SD)
        /// When enabled, attempts to discover services advertised via DNS-SD/mDNS
        service_discovery: bool = true;

        /// TCP ports used to determine if a host is reachable at a given IP address.
        /// If a TCP connection can be established on any of these ports within
        /// the `tcp_port_ms` duration, the host is considered to be up.
        /// TIP: For port descriptions see: <https://www.speedguide.net/ports_common.php>
        tcp_ports: &[u16] = &[22, 80, 443];

        /// Number of I/O threads used for network scanning.
        ///
        /// Each I/O thread is responsible for opening a network socket. If multiple network
        /// interfaces are detected, the total number of threads is distributed evenly among them.
        ///
        /// Valid values:
        /// - 'dynamic': (Default) Queries system resources before each scan and adjusts the
        /// thread count based on CPU availability and system load.
        /// - Range 32-8192: A fixed number of I/O threads will be used for every scan.
        ///
        /// NOTE: This is not a global thread limit. It only constrains the number of
        /// threads performing network I/O.
        io_threads: &str = "dynamic";
    }

    #[section]
    Ui {
        /// Use compact output format
        /// Hides help footer
        compact: bool = false;

        /// Hide IPs with no association (no resolved hostname/service information)
        hide_bare_ips: bool = false;

        /// Maximum number of logs to store in the buffer before logs are dropped
        /// Range 1-4294967295
        log_limit: u32 = 1000;

        /// Log level at startup.
        ///
        /// Logs above the specified log level/verbosity will be suppressed.
        ///
        /// Valid values: error, warn, info, debug, trace
        log_level: &str = "info";
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
