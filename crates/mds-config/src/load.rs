use config::{Config, File};
use mds_cli::Args;
use regex::Regex;
use std::fs;
use std::path::Path;
use toml_edit::DocumentMut;

use crate::AppConfig;
use crate::error::ConfigLoadError;

impl AppConfig {
    /// Load configuration from various sources following Unix CLI conventions
    ///
    /// 1. Built-in defaults
    /// 2. User config (~/.config/mdns-scanner/config.toml)
    /// 3. Local config (./mdns-scanner.toml)
    ///
    /// Default load using OS-dependent paths
    pub fn load() -> Result<Self, ConfigLoadError> {
        let user_path = dirs::config_dir().map(|dir| dir.join("mdns-scanner/config.toml"));
        let local_path = Some(Path::new("mdns-scanner.toml"));

        Self::load_with_paths(user_path.as_deref(), local_path, None)
    }

    /// Load configuration using OS-dependent paths and CLI arguments
    pub fn load_with_cli(cli_args: &Args) -> Result<Self, ConfigLoadError> {
        let user_path = dirs::config_dir().map(|dir| dir.join("mdns-scanner/config.toml"));
        let local_path = Some(Path::new("mdns-scanner.toml"));

        Self::load_with_paths(
            user_path.as_deref(),
            local_path,
            Some(cli_args), // Pass the CLI arguments here
        )
    }

    /// Applies CLI arguments to the AppConfig, giving them the highest precedence.
    /// This function assumes the AppConfig has already been loaded from other sources.
    fn apply_cli_overrides(&mut self, args: &Args) {
        // Only override if the CLI argument was explicitly provided by the user.
        // Clap's `default_value_t` means the field in `args` will *always* have a value.
        // We compare against the AppConfig's internal defaults (mds_default) to determine
        // if the CLI value is a user-supplied override.
        if !args.iface_ignore_re().is_empty() {
            self.iface_ignore_re = args
                .iface_ignore_re()
                .iter()
                .map(|re| re.to_string())
                .collect();
        }

        if let Some(iface_include_docker) = args.iface_include_docker {
            self.iface_include_docker = iface_include_docker;
        }
        if let Some(no_service_discovery) = args.no_service_discovery {
            self.service_discovery = !no_service_discovery;
        }
        if let Some(compact) = args.compact {
            self.compact = compact;
        }
        if let Some(tcp_port_timeout_ms) = args.tcp_port_timeout_ms {
            self.tcp_port_timeout_ms = tcp_port_timeout_ms.get();
        }
        if let Some(ping_timeout_ms) = args.ping_timeout_ms {
            self.ping_timeout_ms = ping_timeout_ms.get();
        }
        if let Some(ip_check_timeout_ms) = args.ip_check_timeout_ms {
            self.ip_check_timeout_ms = ip_check_timeout_ms.get();
        }
    }

    /// Load configuration from various sources with injected file paths
    pub fn load_with_paths(
        user_path: Option<&Path>,
        local_path: Option<&Path>,
        cli_args: Option<&Args>,
    ) -> Result<Self, ConfigLoadError> {
        let mut builder = Config::builder();

        // 1. Built-in defaults
        builder = builder.add_source(Config::try_from(&AppConfig::default())?);

        // 2. User config directory
        if let Some(p) = user_path {
            builder = builder.add_source(
                File::from(p)
                    .format(config::FileFormat::Toml)
                    .required(false),
            );
        }

        // 3. Local project config
        if let Some(p) = local_path {
            builder = builder.add_source(
                File::from(p)
                    .format(config::FileFormat::Toml)
                    .required(false),
            );
        }

        let config = builder.build()?;
        let mut app_config: AppConfig = config.try_deserialize()?;

        // 4. Apply CLI arguments (highest precedence)
        if let Some(args) = cli_args {
            app_config.apply_cli_overrides(args);
        }

        // Compile and cache regex patterns after all sources have been applied
        let compiled_regexes: Vec<Regex> = app_config
            .iface_ignore_re
            .iter()
            .map(|pattern| Regex::new(pattern))
            .collect::<Result<Vec<Regex>, regex::Error>>()?;
        app_config.compiled_iface_ignore_re = Some(compiled_regexes);

        Ok(app_config)
    }

    /// Load configuration from file preserving comments and formatting
    pub fn load_with_comments(
        path: impl AsRef<Path>,
    ) -> Result<(AppConfig, DocumentMut), ConfigLoadError> {
        let content = fs::read_to_string(path)?;
        let config: AppConfig = toml_edit::de::from_str(&content)?;
        let doc = content.parse::<DocumentMut>()?;

        Ok((config, doc))
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, num::NonZeroU16};

    use tempfile::tempdir;
    use testresult::TestResult;

    use super::*;

    #[test]
    fn roundtrip_default_config() -> TestResult {
        let original = AppConfig::default();
        let toml = toml_edit::ser::to_string_pretty(&original)?;
        let parsed: AppConfig = toml_edit::de::from_str(&toml)?;

        assert_eq!(original, parsed);
        Ok(())
    }

    #[test]
    fn loading_config_from_custom_path() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let config_path = temp_dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
            iface_ignore_re = ["eth.*"]
            iface_include_docker = true
            tcp_port_timeout_ms = 200
        "#,
        )
        .unwrap();

        let config = AppConfig::load_with_paths(Some(&config_path), None, None)
            .expect("Failed to load config");

        assert_eq!(config.iface_ignore_re, vec!["eth.*"]);
        assert!(config.iface_include_docker);
        assert_eq!(config.tcp_port_timeout_ms, 200);
        Ok(())
    }

    #[test]
    fn invalid_regex_fails() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("c.toml");
        fs::write(&path, r#"iface_ignore_re = ["*["]"#)?;
        let err = AppConfig::load_with_paths(Some(&path), None, None)
            .expect_err("Should fail due to invalid regex");
        matches!(err, ConfigLoadError::InvalidRegex(_));
        Ok(())
    }

    #[test]
    fn config_precedence_order() -> TestResult {
        let temp = tempfile::tempdir()?;
        let usr = temp.path().join("user.toml");
        let loc = temp.path().join("local.toml");

        fs::write(&usr, r#"tcp_port_timeout_ms = 222"#)?;
        fs::write(&loc, r#"tcp_port_timeout_ms = 333"#)?;

        let cfg = AppConfig::load_with_paths(Some(&usr), Some(&loc), None)?;
        assert_eq!(cfg.tcp_port_timeout_ms, 333);
        Ok(())
    }

    #[test]
    fn partial_config_files() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("partial.toml");
        fs::write(&path, r#"ping_timeout_ms = 123"#)?;
        let cfg = AppConfig::load_with_paths(Some(&path), None, None)?;
        assert_eq!(cfg.ping_timeout_ms, 123);
        assert_eq!(
            cfg.tcp_port_timeout_ms,
            mds_default::TCP_PORT_TIMEOUT_MS.value
        );
        Ok(())
    }

    #[test]
    fn empty_config_file() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("empty.toml");
        fs::write(&path, "")?;
        let cfg = AppConfig::load_with_paths(Some(&path), None, None)?;
        assert_eq!(cfg, AppConfig::default());
        Ok(())
    }

    #[test]
    fn nonexistent_config_files() -> TestResult {
        let cfg = AppConfig::load_with_paths(
            Some(Path::new("nonexistent1.toml")),
            Some(Path::new("nonexistent2.toml")),
            None,
        )?;
        assert_eq!(cfg, AppConfig::default());
        Ok(())
    }

    #[test]
    fn config_with_comments_preservation() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("commented.toml");
        fs::write(
            &path,
            r#"
        # This is a comment
        tcp_port_timeout_ms = 444  # Inline comment
    "#,
        )?;
        let (_cfg, doc) = AppConfig::load_with_comments(&path)?;
        let content = doc.to_string();
        assert!(content.contains("# This is a comment"));
        assert!(content.contains("tcp_port_timeout_ms"));
        Ok(())
    }

    #[test]
    fn cli_precedence_simple_overrides() -> TestResult {
        let temp_dir = tempdir()?;
        let config_path = temp_dir.path().join("config.toml");
        // Config file sets tcp_port_timeout_ms to 500
        fs::write(&config_path, "tcp_port_timeout_ms = 500\ncompact = true")?;

        // CLI args set tcp_port_timeout_ms to 700
        let cli_args = Args {
            tcp_port_timeout_ms: Some(NonZeroU16::new(700).unwrap()),
            // Provide no values
            compact: None,
            iface_ignore_re: vec![],
            iface_include_docker: None,
            no_service_discovery: None,
            ping_timeout_ms: None,
            ip_check_timeout_ms: None,
            command: None,
        };

        let cfg = AppConfig::load_with_paths(Some(&config_path), None, Some(&cli_args))?;

        assert_eq!(
            cfg.tcp_port_timeout_ms, 700,
            "CLI tcp_port_timeout_ms should override file"
        );
        assert!(cfg.compact, "CLI compact should NOT override file");
        Ok(())
    }

    #[test]
    fn cli_vec_and_default_value_overrides() -> TestResult {
        let temp_dir = tempdir()?;
        let config_path = temp_dir.path().join("config.toml");
        // Config file sets iface_ignore_re and ping_timeout_ms
        fs::write(
            &config_path,
            r#"
            iface_ignore_re = ["file_re"]
            ping_timeout_ms = 999
            service_discovery = false
        "#,
        )?;

        // CLI args: new regex, new ping timeout, toggle service discovery
        let cli_args = Args {
            iface_ignore_re: vec![Regex::new("cli_re")?], // Explicit CLI regex
            ping_timeout_ms: Some(NonZeroU16::new(10).unwrap()), // CLI value different from file and default
            no_service_discovery: Some(false), // CLI sets service_discovery to true (overrides false in file)
            // Provide default/dummy values for other Args fields
            iface_include_docker: None,
            compact: None,
            tcp_port_timeout_ms: None,
            ip_check_timeout_ms: None,
            command: None,
        };

        let cfg = AppConfig::load_with_paths(Some(&config_path), None, Some(&cli_args))?;

        assert_eq!(
            cfg.iface_ignore_re,
            vec!["cli_re".to_string()],
            "CLI iface_ignore_re should override"
        );
        assert_eq!(
            cfg.ping_timeout_ms, 10,
            "CLI ping_timeout_ms should override file"
        );
        assert!(
            cfg.service_discovery,
            "CLI no_service_discovery should set service_discovery to true"
        );
        Ok(())
    }
}
