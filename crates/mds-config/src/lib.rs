use config::{Config, ConfigError, Environment, File};
use mds_cli::Args;
use mds_util::host_up::TimeoutSettings;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU16;
use std::path::{Path, PathBuf};
use std::{fs, io};
use thiserror::Error;
use toml_edit::{DocumentMut, Item, Value};

const DEFAULT_CONFIG: &str = include_str!("../../../docs/default_config.toml");

const SYSTEM_PATH: Option<&str> = if cfg!(target_os = "macos") {
    Some("/usr/local/etc/mdns-scanner/config.toml")
} else if cfg!(unix) {
    Some("/etc/mdns-scanner/config.toml")
} else {
    None
};

#[derive(Error, Debug)]
pub enum ConfigLoadError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    #[error("Invalid regex pattern: {0}")]
    InvalidRegex(#[from] regex::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parsing error: {0}")]
    TomlParse(#[from] toml_edit::TomlError),
    #[error("TOML edit error: {0}")]
    TomlEdit(#[from] toml_edit::de::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    iface_ignore_re: Vec<String>,
    iface_include_docker: bool,
    service_discovery: bool,
    compact: bool,
    tcp_port_timeout_ms: u16,
    ping_timeout_ms: u16,
    ip_check_timeout_ms: u16,
    #[serde(skip)]
    compiled_iface_ignore_re: Option<Vec<Regex>>, // Cached compiled regexes
}

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

impl AppConfig {
    /// Load configuration from various sources following Unix CLI conventions
    ///
    /// 1. Built-in defaults
    /// 2. System-wide config (/etc/mdns-scanner/config.toml)
    /// 3. User config (~/.config/mdns-scanner/config.toml or ~/.mdns-scanner.toml)
    /// 4. Local config (./mdns-scanner.toml)
    /// 5. Environment variables (MDNS_SCANNER_*)
    ///
    /// Default load using OS-dependent paths
    pub fn load() -> Result<Self, ConfigLoadError> {
        let system_path = SYSTEM_PATH.map(Path::new);

        let user_path = dirs::config_dir().map(|dir| dir.join("mdns-scanner/config.toml"));
        let home_path = dirs::home_dir().map(|dir| dir.join(".mdns-scanner.toml"));
        let local_path = Some(Path::new("mdns-scanner.toml"));

        Self::load_with_paths(
            system_path,
            user_path.as_deref(),
            home_path.as_deref(),
            local_path,
            None,
        )
    }

    /// Load configuration using OS-dependent paths and CLI arguments
    pub fn load_with_cli(cli_args: &Args) -> Result<Self, ConfigLoadError> {
        let system_path = SYSTEM_PATH.map(Path::new);

        let user_path = dirs::config_dir().map(|dir| dir.join("mdns-scanner/config.toml"));
        let home_path = dirs::home_dir().map(|dir| dir.join(".mdns-scanner.toml"));
        let local_path = Some(Path::new("mdns-scanner.toml"));

        Self::load_with_paths(
            system_path,
            user_path.as_deref(),
            home_path.as_deref(),
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
        system_path: Option<&Path>,
        user_path: Option<&Path>,
        home_path: Option<&Path>,
        local_path: Option<&Path>,
        cli_args: Option<&Args>,
    ) -> Result<Self, ConfigLoadError> {
        let mut builder = Config::builder();

        // 1. Built-in defaults
        builder = builder.add_source(Config::try_from(&AppConfig::default())?);

        // 2. System-wide config
        if let Some(p) = system_path {
            builder = builder.add_source(
                File::from(p)
                    .format(config::FileFormat::Toml)
                    .required(false),
            );
        }

        // 3. User config directory
        if let Some(p) = user_path {
            builder = builder.add_source(
                File::from(p)
                    .format(config::FileFormat::Toml)
                    .required(false),
            );
        }

        // 4. Home fallback
        if let Some(p) = home_path {
            builder = builder.add_source(
                File::from(p)
                    .format(config::FileFormat::Toml)
                    .required(false),
            );
        }

        // 5. Local project config
        if let Some(p) = local_path {
            builder = builder.add_source(
                File::from(p)
                    .format(config::FileFormat::Toml)
                    .required(false),
            );
        }

        // 6. Environment variables (optional for safety; can be disabled in tests)
        builder = builder.add_source(
            Environment::with_prefix("MDS")
                .separator("_")
                .try_parsing(true),
        );

        let config = builder.build()?;
        let mut app_config: AppConfig = config.try_deserialize()?;

        // 7. Apply CLI arguments (highest precedence)
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

    /// Get compiled regex patterns for interface ignoring from the cache.
    /// Panics if called before config is loaded and regexes are compiled.
    pub fn iface_ignore_regex(&self) -> &[Regex] {
        self.compiled_iface_ignore_re
            .as_ref()
            .expect("iface_ignore_regex called before AppConfig was fully loaded and compiled.")
    }

    /// Get TCP port timeout
    pub fn tcp_port_timeout(&self) -> Option<NonZeroU16> {
        NonZeroU16::new(self.tcp_port_timeout_ms)
    }

    /// Get ping timeout
    pub fn ping_timeout(&self) -> Option<NonZeroU16> {
        NonZeroU16::new(self.ping_timeout_ms)
    }

    /// Get IP check timeout
    pub fn ip_check_timeout(&self) -> Option<NonZeroU16> {
        NonZeroU16::new(self.ip_check_timeout_ms)
    }

    pub fn iface_include_docker(&self) -> bool {
        self.iface_include_docker
    }

    pub fn compact(&self) -> bool {
        self.compact
    }

    pub fn timeout_settings(&self) -> TimeoutSettings {
        TimeoutSettings {
            tcp_port_timeout_ms: self.tcp_port_timeout().unwrap(),
            ping_timeout_ms: self.ping_timeout().unwrap(),
            ip_check_timeout_ms: self.ip_check_timeout().unwrap(),
        }
    }

    /// Get service discovery enabled (inverted from CLI's no_service_discovery)
    pub fn service_discovery_enabled(&self) -> bool {
        self.service_discovery
    }

    /// Get the user config file path
    pub fn user_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("mdns-scanner").join("config.toml"))
    }

    /// Write the default configuration to the user config directory
    pub fn write_default_config() -> Result<PathBuf, ConfigLoadError> {
        let config_path = Self::user_config_path().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine user config directory",
            )
        })?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&config_path, DEFAULT_CONFIG)?;

        Ok(config_path)
    }

    /// Return the default config as a string
    pub fn default_config() -> &'static str {
        DEFAULT_CONFIG
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

    /// Update a TOML document with new values while preserving comments
    pub fn update_document(
        doc: &mut DocumentMut,
        config: &AppConfig,
    ) -> Result<(), ConfigLoadError> {
        // Update each field, preserving existing comments and structure
        let iface_ignore_re = mds_default::IFACE_IGNORE_RE.key;
        if let Some(array) = doc[iface_ignore_re].as_array_mut() {
            array.clear();
            for pattern in &config.iface_ignore_re {
                array.push(pattern.as_str());
            }
        } else {
            doc[iface_ignore_re] = Item::Value({
                let mut arr = toml_edit::Array::new();
                for pattern in &config.iface_ignore_re {
                    arr.push(pattern.as_str());
                }
                Value::Array(arr)
            });
        }

        doc[mds_default::IFACE_INCLUDE_DOCKER.key] = Item::Value(Value::Boolean(
            toml_edit::Formatted::new(config.iface_include_docker),
        ));
        doc[mds_default::SERVICE_DISCOVERY.key] = Item::Value(Value::Boolean(
            toml_edit::Formatted::new(config.service_discovery),
        ));
        doc[mds_default::COMPACT.key] =
            Item::Value(Value::Boolean(toml_edit::Formatted::new(config.compact)));
        doc[mds_default::TCP_PORT_TIMEOUT_MS.key] = Item::Value(Value::Integer(
            toml_edit::Formatted::new(config.tcp_port_timeout_ms as i64),
        ));
        doc[mds_default::PING_TIMEOUT_MS.key] = Item::Value(Value::Integer(
            toml_edit::Formatted::new(config.ping_timeout_ms as i64),
        ));
        doc[mds_default::IP_CHECK_TIMEOUT_MS.key] = Item::Value(Value::Integer(
            toml_edit::Formatted::new(config.ip_check_timeout_ms as i64),
        ));

        Ok(())
    }

    /// Save configuration to file while preserving comments and formatting
    pub fn save_with_comments(
        path: impl AsRef<Path>,
        config: &AppConfig,
        doc: Option<DocumentMut>,
    ) -> Result<(), ConfigLoadError> {
        let document = if let Some(mut doc) = doc {
            Self::update_document(&mut doc, config)?;
            doc
        } else {
            let toml_string = toml_edit::ser::to_string_pretty(config)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            toml_string.parse::<DocumentMut>()?
        };

        fs::write(path, document.to_string())?;
        Ok(())
    }

    /// Load, modify, and save config while preserving comments
    pub fn modify_user_config<F>(modifier: F) -> Result<PathBuf, ConfigLoadError>
    where
        F: FnOnce(&mut AppConfig),
    {
        let config_path = Self::user_config_path().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine user config directory",
            )
        })?;

        let (mut config, doc) = if config_path.exists() {
            Self::load_with_comments(&config_path)?
        } else {
            // Create default config first
            Self::write_default_config()?;
            Self::load_with_comments(&config_path)?
        };

        // Apply modifications
        modifier(&mut config);

        Self::save_with_comments(&config_path, &config, Some(doc))?;

        Ok(config_path)
    }
}

#[cfg(test)]
mod tests {
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

        let config = AppConfig::load_with_paths(None, Some(&config_path), None, None, None)
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
        let err = AppConfig::load_with_paths(None, Some(&path), None, None, None)
            .expect_err("Should fail due to invalid regex");
        matches!(err, ConfigLoadError::InvalidRegex(_));
        Ok(())
    }

    #[test]
    fn config_precedence_order() -> TestResult {
        let temp = tempfile::tempdir()?;
        let sys = temp.path().join("system.toml");
        let usr = temp.path().join("user.toml");
        let loc = temp.path().join("local.toml");

        fs::write(&sys, r#"tcp_port_timeout_ms = 111"#)?;
        fs::write(&usr, r#"tcp_port_timeout_ms = 222"#)?;
        fs::write(&loc, r#"tcp_port_timeout_ms = 333"#)?;

        let cfg = AppConfig::load_with_paths(Some(&sys), Some(&usr), None, Some(&loc), None)?;
        assert_eq!(cfg.tcp_port_timeout_ms, 333);
        Ok(())
    }

    #[test]
    fn partial_config_files() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("partial.toml");
        fs::write(&path, r#"ping_timeout_ms = 123"#)?;
        let cfg = AppConfig::load_with_paths(None, Some(&path), None, None, None)?;
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
        let cfg = AppConfig::load_with_paths(None, Some(&path), None, None, None)?;
        assert_eq!(cfg, AppConfig::default());
        Ok(())
    }

    #[test]
    fn nonexistent_config_files() -> TestResult {
        let cfg = AppConfig::load_with_paths(
            Some(Path::new("nonexistent1.toml")),
            Some(Path::new("nonexistent2.toml")),
            Some(Path::new("nonexistent3.toml")),
            Some(Path::new("nonexistent4.toml")),
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
    fn save_with_comments() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("save.toml");
        fs::write(
            &path,
            r#"
        # Original comment
        compact = false
    "#,
        )?;

        let (mut cfg, doc) = AppConfig::load_with_comments(&path)?;
        cfg.compact = true;
        AppConfig::save_with_comments(&path, &cfg, Some(doc))?;

        let updated = fs::read_to_string(&path)?;
        assert!(updated.contains("compact = true"));
        assert!(updated.contains("# Original comment"));
        Ok(())
    }

    #[test]
    fn modify_user_config_creates_default_safe_and_preserves_comments() -> TestResult {
        let dir = tempfile::tempdir()?;
        let config_path = dir.path().join("mdns-scanner/config.toml");

        fs::create_dir_all(config_path.parent().unwrap())?;

        // Create a config with comments
        let original = r#"
        # This is a comment before iface_include_docker
        iface_include_docker = false

        # Timeout for TCP port checks
        tcp_port_timeout_ms = 150
    "#;

        fs::write(&config_path, original)?;

        // Load, modify, and save
        let (mut config, doc) = AppConfig::load_with_comments(&config_path)?;
        assert!(!config.iface_include_docker);
        config.iface_include_docker = true;
        AppConfig::save_with_comments(&config_path, &config, Some(doc))?;

        // Reload as plain text and check for comment preservation
        let updated_content = fs::read_to_string(&config_path)?;
        assert!(updated_content.contains("# This is a comment before iface_include_docker"));
        assert!(updated_content.contains("# Timeout for TCP port checks"));
        assert!(updated_content.contains("iface_include_docker = true"));

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

        let cfg =
            AppConfig::load_with_paths(None, Some(&config_path), None, None, Some(&cli_args))?;

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

        let cfg =
            AppConfig::load_with_paths(None, Some(&config_path), None, None, Some(&cli_args))?;

        assert_eq!(
            cfg.iface_ignore_re,
            vec!["cli_re".to_string()],
            "CLI iface_ignore_re (Vec) should override"
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
